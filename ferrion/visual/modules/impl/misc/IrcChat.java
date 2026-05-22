package ferrion.visual.modules.impl.misc;

import ferrion.visual.events.api.EventHandler;
import ferrion.visual.events.impl.KeyEvent;
import ferrion.visual.events.impl.TickEvent;
import ferrion.visual.modules.impl.misc.irc.IrcChatClient;
import ferrion.visual.modules.impl.misc.irc.IrcDefaults;
import ferrion.visual.modules.impl.misc.irc.IrcMessage;
import ferrion.visual.modules.module.ModuleStructure;
import ferrion.visual.modules.module.category.ModuleCategory;
import ferrion.visual.modules.module.setting.implement.BindSetting;
import ferrion.visual.modules.module.setting.implement.BooleanSetting;
import ferrion.visual.modules.module.setting.implement.ButtonSetting;
import ferrion.visual.modules.module.setting.implement.TextSetting;
import ferrion.visual.screens.irc.IrcChatScreen;
import org.lwjgl.glfw.GLFW;

import java.io.IOException;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;

public class IrcChat extends ModuleStructure {
    private static final int MAX_HISTORY = 120;
    private static final long CONNECT_RETRY_MS = 5000L;
    private static final DateTimeFormatter TIME_FORMAT = DateTimeFormatter.ofPattern("HH:mm");

    private final TextSetting nickname = new TextSetting("Nickname", "Chat nickname")
            .setText("Player")
            .setMax(32);
    private final BindSetting chatBind = new BindSetting("Chat Bind", "Opens IRC chat screen")
            .setKey(GLFW.GLFW_KEY_UNKNOWN);
    private final BooleanSetting autoConnect = new BooleanSetting("Auto Connect", "Connect when module is enabled")
            .setValue(IrcDefaults.AUTO_CONNECT);
    private final ButtonSetting reconnect = new ButtonSetting("Reconnect", "Reconnect to IRC server")
            .setButtonName("Reconnect")
            .setRunnable(this::reconnect);
    private final ButtonSetting openChat = new ButtonSetting("Open Chat", "Open IRC chat screen")
            .setButtonName("Open")
            .setRunnable(this::openScreen);

    private final List<IrcMessage> history = new ArrayList<>();
    private final AtomicBoolean connecting = new AtomicBoolean(false);
    private IrcChatClient client;
    private long lastConnectAttempt;

    public IrcChat() {
        super("IRC Chat", "TCP client chat", ModuleCategory.MISC);
        settings(nickname, chatBind, autoConnect, reconnect, openChat);
    }

    @Override
    public void activate() {
        if (autoConnect.isValue()) {
            connectAsync(false);
        }
    }

    @Override
    public void deactivate() {
        disconnect();
    }

    @EventHandler
    public void onKey(KeyEvent event) {
        if (!event.isKeyDown(chatBind.getKey())) return;
        if (chatBind.getKey() == GLFW.GLFW_KEY_UNKNOWN || chatBind.getKey() == -1) return;
        openScreen();
    }

    @EventHandler
    public void onTick(TickEvent event) {
        if (!isState() || !autoConnect.isValue() || isConnected() || isConnecting()) return;
        if (System.currentTimeMillis() - lastConnectAttempt < CONNECT_RETRY_MS) return;
        connectAsync(false);
    }

    public void openScreen() {
        if (mc.player == null || mc.world == null || !isState()) return;
        if (autoConnect.isValue() && !isConnected() && !isConnecting()) {
            connectAsync(true);
        }
        mc.setScreen(new IrcChatScreen(this));
    }

    public void reconnect() {
        disconnect();
        connectAsync(true);
    }

    public void connect() {
        connectAsync(true);
    }

    private void connectAsync(boolean userRequested) {
        if (isConnected()) return;
        if (!connecting.compareAndSet(false, true)) return;

        String host = getServerIp();
        if (host.isBlank()) {
            addSystemMessage("Server IP is empty");
            connecting.set(false);
            return;
        }

        int targetPort = getPort();
        lastConnectAttempt = System.currentTimeMillis();
        Thread thread = new Thread(() -> {
            try {
                IrcChatClient newClient = new IrcChatClient(this::addMessage);
                newClient.connect(host, targetPort, getNickname());
                client = newClient;
            } catch (IOException e) {
                addSystemMessage("Connection failed: " + e.getMessage());
            } finally {
                connecting.set(false);
            }
        }, "ferrion-irc-connect");
        thread.setDaemon(true);
        thread.start();
    }

    public void disconnect() {
        connecting.set(false);
        IrcChatClient current = client;
        client = null;
        if (current != null) {
            current.disconnect();
        }
    }

    public void sendMessage(String text) {
        String trimmed = text == null ? "" : text.trim();
        if (trimmed.isEmpty()) return;

        IrcChatClient current = client;
        if (current == null || !current.isConnected()) {
            if (autoConnect.isValue() && !isConnecting()) {
                connectAsync(true);
                addSystemMessage("Message was not sent. Wait for online status and send again.");
            } else {
                addSystemMessage("Not connected");
            }
            return;
        }

        try {
            current.sendMessage(trimmed);
        } catch (IOException e) {
            addSystemMessage("Send failed: " + e.getMessage());
        }
    }

    public boolean isConnected() {
        IrcChatClient current = client;
        return current != null && current.isConnected();
    }

    public boolean isConnecting() {
        return connecting.get();
    }

    public List<IrcMessage> getHistorySnapshot() {
        synchronized (history) {
            return new ArrayList<>(history);
        }
    }

    public String getServerIp() {
        return IrcDefaults.HOST;
    }

    public int getPort() {
        return IrcDefaults.PORT;
    }

    public String getNickname() {
        String value = nickname.getText() == null ? "" : nickname.getText().trim();
        if (!value.isEmpty()) return value;
        if (mc.getSession() != null && mc.getSession().getUsername() != null) {
            return mc.getSession().getUsername();
        }
        return "Player";
    }

    private void addMessage(IrcMessage message) {
        if (message == null || message.isConnectionNoise()) return;
        synchronized (history) {
            history.add(message);
            while (history.size() > MAX_HISTORY) {
                history.removeFirst();
            }
        }
    }

    private void addSystemMessage(String text) {
        addMessage(IrcMessage.system("[" + LocalTime.now().format(TIME_FORMAT) + "] " + text));
    }
}
