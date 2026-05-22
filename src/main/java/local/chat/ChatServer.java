package local.chat;

import java.io.BufferedWriter;
import java.io.IOException;
import java.io.OutputStreamWriter;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public final class ChatServer {
    public static final int DEFAULT_PORT = 20036;
    public static final int DEFAULT_MAX_CLIENTS = 100;

    private final int requestedPort;
    private final MessageBroadcaster broadcaster;
    private final AtomicBoolean running = new AtomicBoolean(true);
    private final CountDownLatch started = new CountDownLatch(1);
    private final Object socketLock = new Object();

    private volatile ServerSocket serverSocket;
    private volatile IOException startupFailure;

    public ChatServer(int port, int maxClients) {
        if (port < 0 || port > 65535) {
            throw new IllegalArgumentException("port must be between 0 and 65535");
        }
        this.requestedPort = port;
        this.broadcaster = new MessageBroadcaster(maxClients);
    }

    public static void main(String[] args) {
        int port = parsePort(args);
        new ChatServer(port, DEFAULT_MAX_CLIENTS).start();
    }

    public static ChatServer startInBackground(int port, int maxClients) throws InterruptedException, IOException {
        ChatServer server = new ChatServer(port, maxClients);
        Thread thread = new Thread(server::start, "chat-server-accept");
        thread.setDaemon(true);
        thread.start();
        server.awaitStarted();
        return server;
    }

    public void start() {
        try (ServerSocket openedSocket = new ServerSocket()) {
            openedSocket.setReuseAddress(true);
            openedSocket.bind(new InetSocketAddress(requestedPort));
            synchronized (socketLock) {
                serverSocket = openedSocket;
            }
            System.out.println("Chat server listening on port " + port());
            started.countDown();

            while (running.get()) {
                acceptClient(openedSocket);
            }
        } catch (IOException e) {
            startupFailure = e;
            if (running.get()) {
                System.err.println("Server stopped with error: " + e.getMessage());
            }
        } finally {
            started.countDown();
            broadcaster.closeAll();
        }
    }

    public int port() {
        ServerSocket current = serverSocket;
        if (current == null) {
            throw new IllegalStateException("server is not started");
        }
        return current.getLocalPort();
    }

    public void stop() throws InterruptedException {
        running.set(false);
        ServerSocket current = serverSocket;
        if (current != null) {
            try {
                current.close();
            } catch (IOException ignored) {
                // Closing during shutdown is best-effort.
            }
        }
        broadcaster.closeAll();
        started.await(3, TimeUnit.SECONDS);
    }

    private static int parsePort(String[] args) {
        if (args.length == 0) {
            return DEFAULT_PORT;
        }
        if (args.length > 1) {
            throw new IllegalArgumentException("Usage: java -jar chat-server.jar [port]");
        }
        try {
            int port = Integer.parseInt(args[0]);
            if (port < 1 || port > 65535) {
                throw new IllegalArgumentException("Port must be between 1 and 65535");
            }
            return port;
        } catch (NumberFormatException e) {
            throw new IllegalArgumentException("Port must be a number", e);
        }
    }

    private void awaitStarted() throws InterruptedException, IOException {
        if (!started.await(10, TimeUnit.SECONDS)) {
            throw new IOException("Timed out waiting for server startup");
        }
        if (startupFailure != null) {
            throw startupFailure;
        }
    }

    private void acceptClient(ServerSocket socket) throws IOException {
        Socket clientSocket = socket.accept();
        clientSocket.setTcpNoDelay(true);

        ClientSession session;
        try {
            session = new ClientSession(clientSocket, broadcaster);
        } catch (IOException e) {
            clientSocket.close();
            throw e;
        }

        if (!broadcaster.register(session)) {
            System.out.println("Rejected connection from " + clientSocket.getRemoteSocketAddress() + ": server full");
            sendServerFullAndClose(clientSocket);
            return;
        }

        Thread thread = new Thread(session, "client-session-" + clientSocket.getRemoteSocketAddress());
        thread.start();
    }

    private static void sendServerFullAndClose(Socket socket) {
        try (socket;
                BufferedWriter writer = new BufferedWriter(
                        new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8))) {
            writer.write("SYSTEM|Server full");
            writer.newLine();
            writer.flush();
        } catch (IOException ignored) {
            // The client may already have disconnected.
        }
    }
}
