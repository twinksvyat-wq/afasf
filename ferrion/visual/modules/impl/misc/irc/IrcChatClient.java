package ferrion.visual.modules.impl.misc.irc;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.function.Consumer;

public final class IrcChatClient {
    private static final int CONNECT_TIMEOUT_MS = 5000;

    private final Consumer<IrcMessage> listener;
    private final Object writerLock = new Object();

    private volatile Socket socket;
    private volatile BufferedWriter writer;
    private volatile Thread readerThread;
    private volatile boolean connected;

    public IrcChatClient(Consumer<IrcMessage> listener) {
        this.listener = listener;
    }

    public void connect(String host, int port, String nickname) throws IOException {
        disconnect();

        Socket openedSocket = new Socket();
        openedSocket.setTcpNoDelay(true);
        openedSocket.connect(new InetSocketAddress(host, port), CONNECT_TIMEOUT_MS);

        BufferedWriter openedWriter = new BufferedWriter(
                new OutputStreamWriter(openedSocket.getOutputStream(), StandardCharsets.UTF_8));
        BufferedReader reader = new BufferedReader(
                new InputStreamReader(openedSocket.getInputStream(), StandardCharsets.UTF_8));

        socket = openedSocket;
        writer = openedWriter;
        connected = true;
        sendRaw("HELLO|" + sanitizeNickname(nickname));

        Thread thread = new Thread(() -> readLoop(reader), "ferrion-irc-chat-reader");
        thread.setDaemon(true);
        readerThread = thread;
        thread.start();
    }

    public void sendMessage(String text) throws IOException {
        if (!isConnected()) throw new IOException("Not connected");
        sendRaw("MSG|" + sanitizeMessage(text));
    }

    public void disconnect() {
        connected = false;
        Socket currentSocket = socket;
        socket = null;
        writer = null;
        if (currentSocket != null) {
            try {
                currentSocket.close();
            } catch (IOException ignored) {
                // Socket may already be closed by the reader thread.
            }
        }
    }

    public boolean isConnected() {
        Socket currentSocket = socket;
        return connected && currentSocket != null && currentSocket.isConnected() && !currentSocket.isClosed();
    }

    private void readLoop(BufferedReader reader) {
        try {
            String line;
            while (connected && (line = reader.readLine()) != null) {
                listener.accept(IrcMessage.parse(line));
            }
        } catch (IOException e) {
            if (connected) {
                listener.accept(IrcMessage.system("Disconnected: " + e.getMessage()));
            }
        } finally {
            disconnect();
        }
    }

    private void sendRaw(String line) throws IOException {
        BufferedWriter currentWriter = writer;
        if (currentWriter == null) throw new IOException("Not connected");
        synchronized (writerLock) {
            currentWriter.write(line);
            currentWriter.newLine();
            currentWriter.flush();
        }
    }

    private String sanitizeNickname(String nickname) {
        String trimmed = nickname == null ? "" : nickname.trim();
        if (trimmed.isEmpty()) return "Player";
        return trimmed.replace('|', '_').substring(0, Math.min(trimmed.length(), 32));
    }

    private String sanitizeMessage(String text) {
        String normalized = text == null ? "" : text.replace('\n', ' ').replace('\r', ' ');
        return normalized.length() <= 200 ? normalized : normalized.substring(0, 200);
    }
}
