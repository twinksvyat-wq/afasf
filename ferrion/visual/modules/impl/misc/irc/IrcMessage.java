package ferrion.visual.modules.impl.misc.irc;

import java.util.Locale;

public record IrcMessage(Kind kind, String nickname, String text) {
    public enum Kind {
        CHAT,
        SYSTEM
    }

    private static final String CHAT_PREFIX = "CHAT|";
    private static final String SYSTEM_PREFIX = "SYSTEM|";

    public static IrcMessage parse(String line) {
        if (line == null || line.isBlank()) {
            return system("");
        }
        if (line.startsWith(CHAT_PREFIX)) {
            String payload = line.substring(CHAT_PREFIX.length());
            int separator = payload.indexOf('|');
            if (separator > -1) {
                return new IrcMessage(Kind.CHAT, payload.substring(0, separator), payload.substring(separator + 1));
            }
        }
        if (line.startsWith(SYSTEM_PREFIX)) {
            return system(line.substring(SYSTEM_PREFIX.length()));
        }
        return system(line);
    }

    public static IrcMessage system(String text) {
        return new IrcMessage(Kind.SYSTEM, "", text == null ? "" : text);
    }

    public boolean isConnectionNoise() {
        if (kind != Kind.SYSTEM) return false;
        String value = text == null ? "" : text.trim().toLowerCase(Locale.ROOT);
        return value.equals("connected")
                || value.equals("disconnected")
                || value.startsWith("connected to ")
                || value.startsWith("connecting to ")
                || value.startsWith("auto connecting to ");
    }
}
