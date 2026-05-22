package local.chat;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.time.Clock;
import java.time.Duration;
import java.util.concurrent.atomic.AtomicBoolean;

public final class ClientSession implements Runnable {
    private static final int MAX_MESSAGE_LENGTH = 200;

    private final Socket socket;
    private final MessageBroadcaster broadcaster;
    private final RateLimiter rateLimiter;
    private final BufferedReader reader;
    private final BufferedWriter writer;
    private final AtomicBoolean closed = new AtomicBoolean(false);

    private volatile String nickname;

    public ClientSession(Socket socket, MessageBroadcaster broadcaster) throws IOException {
        this.socket = socket;
        this.broadcaster = broadcaster;
        this.rateLimiter = new RateLimiter(Duration.ofSeconds(1), Clock.systemUTC());
        this.reader = new BufferedReader(
                new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
        this.writer = new BufferedWriter(
                new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8));
    }

    @Override
    public void run() {
        String remoteAddress = socket.getRemoteSocketAddress().toString();
        System.out.println("Client connected: " + remoteAddress);
        sendLine("SYSTEM|Connected");

        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (!handleLine(line)) {
                    break;
                }
            }
        } catch (IOException e) {
            if (!closed.get()) {
                System.out.println("Client read failed: " + remoteAddress + " - " + e.getMessage());
            }
        } finally {
            disconnect(remoteAddress);
        }
    }

    public void sendLine(String line) {
        synchronized (writer) {
            try {
                writer.write(line);
                writer.newLine();
                writer.flush();
            } catch (IOException e) {
                close();
            }
        }
    }

    public void close() {
        if (closed.compareAndSet(false, true)) {
            try {
                socket.close();
            } catch (IOException ignored) {
                // Closing an already-broken socket is harmless.
            }
        }
    }

    private boolean handleLine(String line) {
        if (line.startsWith("HELLO|")) {
            handleHello(line.substring("HELLO|".length()));
            return true;
        }
        if (line.startsWith("MSG|")) {
            handleMessage(line.substring("MSG|".length()));
            return true;
        }
        if (line.equals("PING")) {
            sendLine("PONG");
            return true;
        }
        if (line.equals("QUIT")) {
            return false;
        }

        System.out.println("Rejected invalid command from " + displayName() + ": " + line);
        sendLine("SYSTEM|Invalid command");
        return true;
    }

    private void handleHello(String rawNickname) {
        String trimmed = rawNickname.trim();
        if (trimmed.isEmpty()) {
            System.out.println("Rejected invalid nickname from " + socket.getRemoteSocketAddress());
            sendLine("SYSTEM|Invalid nickname");
            return;
        }
        nickname = trimmed;
        System.out.println("Client identified: " + trimmed);
    }

    private void handleMessage(String text) {
        String currentNickname = nickname;
        if (currentNickname == null) {
            System.out.println("Rejected message without nickname from " + socket.getRemoteSocketAddress());
            sendLine("SYSTEM|Nickname required");
            return;
        }
        if (text.length() > MAX_MESSAGE_LENGTH) {
            System.out.println("Rejected long message from " + currentNickname);
            sendLine("SYSTEM|Message too long");
            return;
        }
        if (!rateLimiter.tryAcquire()) {
            System.out.println("Rejected spam from " + currentNickname);
            sendLine("SYSTEM|Rate limit exceeded");
            return;
        }

        System.out.println("Message from " + currentNickname + ": " + text);
        broadcaster.broadcast("CHAT|" + currentNickname + "|" + text);
    }

    private void disconnect(String remoteAddress) {
        boolean named = nickname != null;
        String disconnectedName = nickname;
        close();
        broadcaster.remove(this);
        System.out.println("Client disconnected: " + (named ? disconnectedName : remoteAddress));
        if (named) {
            broadcaster.broadcast("SYSTEM|" + disconnectedName + " disconnected");
        }
    }

    private String displayName() {
        String currentNickname = nickname;
        return currentNickname == null ? socket.getRemoteSocketAddress().toString() : currentNickname;
    }
}
