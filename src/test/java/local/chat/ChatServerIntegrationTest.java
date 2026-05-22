package local.chat;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.Closeable;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

class ChatServerIntegrationTest {
    private ChatServer server;

    @AfterEach
    void stopServer() throws Exception {
        if (server != null) {
            server.stop();
        }
    }

    @Test
    void sendsConnectedMessageOnConnect() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient client = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", client.readLine());
        }
    }

    @Test
    void broadcastsChatMessagesWithNickname() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient alice = TestClient.connect(server.port());
                TestClient bob = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", alice.readLine());
            assertEquals("SYSTEM|Connected", bob.readLine());

            alice.writeLine("HELLO|Alice");
            alice.writeLine("MSG|hello everyone");

            assertEquals("CHAT|Alice|hello everyone", alice.readLine());
            assertEquals("CHAT|Alice|hello everyone", bob.readLine());
        }
    }

    @Test
    void respondsToPing() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient client = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", client.readLine());
            client.writeLine("PING");

            assertEquals("PONG", client.readLine());
        }
    }

    @Test
    void rejectsMessagesLongerThanTwoHundredCharacters() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient client = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", client.readLine());
            client.writeLine("HELLO|Alice");
            client.writeLine("MSG|" + "a".repeat(201));

            assertEquals("SYSTEM|Message too long", client.readLine());
        }
    }

    @Test
    void rejectsSpamWithinOneSecond() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient client = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", client.readLine());
            client.writeLine("HELLO|Alice");
            client.writeLine("MSG|first");
            assertEquals("CHAT|Alice|first", client.readLine());

            client.writeLine("MSG|second");

            assertEquals("SYSTEM|Rate limit exceeded", client.readLine());
        }
    }

    @Test
    void broadcastsNamedClientDisconnectAndKeepsOtherClientsConnected() throws Exception {
        server = ChatServer.startInBackground(0, 100);

        try (TestClient alice = TestClient.connect(server.port());
                TestClient bob = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", alice.readLine());
            assertEquals("SYSTEM|Connected", bob.readLine());
            alice.writeLine("HELLO|Alice");

            alice.close();

            assertEquals("SYSTEM|Alice disconnected", bob.readLine());
            bob.writeLine("HELLO|Bob");
            bob.writeLine("MSG|still here");
            assertEquals("CHAT|Bob|still here", bob.readLine());
        }
    }

    @Test
    void rejectsConnectionsWhenMaximumClientsReached() throws Exception {
        server = ChatServer.startInBackground(0, 1);

        try (TestClient first = TestClient.connect(server.port());
                TestClient second = TestClient.connect(server.port())) {
            assertEquals("SYSTEM|Connected", first.readLine());
            assertEquals("SYSTEM|Server full", second.readLine());
            assertTrue(second.isClosedSoon());
        }
    }

    private static final class TestClient implements Closeable {
        private final Socket socket;
        private final BufferedReader reader;
        private final BufferedWriter writer;

        private TestClient(Socket socket) throws IOException {
            this.socket = socket;
            this.socket.setSoTimeout((int) Duration.ofSeconds(3).toMillis());
            this.reader = new BufferedReader(
                    new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
            this.writer = new BufferedWriter(
                    new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8));
        }

        private static TestClient connect(int port) throws IOException {
            return new TestClient(new Socket("127.0.0.1", port));
        }

        private String readLine() throws IOException {
            return reader.readLine();
        }

        private void writeLine(String line) throws IOException {
            writer.write(line);
            writer.newLine();
            writer.flush();
        }

        private boolean isClosedSoon() throws IOException {
            return reader.readLine() == null;
        }

        @Override
        public void close() throws IOException {
            socket.close();
        }
    }
}
