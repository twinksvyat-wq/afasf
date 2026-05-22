package local.chat;

import java.util.ArrayList;
import java.util.List;

public final class MessageBroadcaster {
    private final int maxClients;
    private final List<ClientSession> clients = new ArrayList<>();

    public MessageBroadcaster(int maxClients) {
        if (maxClients < 1) {
            throw new IllegalArgumentException("maxClients must be positive");
        }
        this.maxClients = maxClients;
    }

    public synchronized boolean register(ClientSession session) {
        if (clients.size() >= maxClients) {
            return false;
        }
        clients.add(session);
        return true;
    }

    public synchronized void remove(ClientSession session) {
        clients.remove(session);
    }

    public void broadcast(String line) {
        List<ClientSession> snapshot;
        synchronized (this) {
            snapshot = List.copyOf(clients);
        }
        for (ClientSession client : snapshot) {
            client.sendLine(line);
        }
    }

    public void closeAll() {
        List<ClientSession> snapshot;
        synchronized (this) {
            snapshot = List.copyOf(clients);
        }
        for (ClientSession client : snapshot) {
            client.close();
        }
    }

    public synchronized int clientCount() {
        return clients.size();
    }
}
