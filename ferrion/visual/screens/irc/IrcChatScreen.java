package ferrion.visual.screens.irc;

import ferrion.visual.modules.impl.misc.IrcChat;
import ferrion.visual.modules.impl.misc.irc.IrcDefaults;
import ferrion.visual.modules.impl.misc.irc.IrcMessage;
import ferrion.visual.screens.clickgui.dp.DpClickGuiStyle;
import ferrion.visual.util.render.Render2D;
import ferrion.visual.util.render.font.Fonts;
import ferrion.visual.util.render.shader.Scissor;
import net.minecraft.client.gui.Click;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.input.CharInput;
import net.minecraft.client.input.KeyInput;
import net.minecraft.client.render.RenderTickCounter;
import net.minecraft.text.Text;
import net.minecraft.util.Identifier;
import org.lwjgl.glfw.GLFW;

import java.util.List;

public class IrcChatScreen extends Screen {
    private static final int FIXED_GUI_SCALE = 2;
    private static final Identifier CHAT_ICON = Identifier.of("ferrion.visual", "textures/dropdown/misc.png");
    private static final Identifier GLOW_TEXTURE = Identifier.of("ferrion.visual", "images/particle/glow.png");

    private static final float PANEL_WIDTH = 378f;
    private static final float PANEL_HEIGHT = 226f;
    private static final float HEADER_HEIGHT = 30f;
    private static final float INPUT_HEIGHT = 24f;
    private static final float PADDING = 8f;
    private static final float LINE_HEIGHT = 15f;
    private static final float ICON_SIZE = 10f;
    private static final float TITLE_SIZE = 7.2f;
    private static final float META_SIZE = 5.3f;
    private static final float MESSAGE_SIZE = 5.7f;
    private static final float NICK_SIZE = 5.8f;
    private static final float INPUT_SIZE = 5.9f;

    private final IrcChat module;
    private String input = "";
    private int scrollOffset;
    private int lastMouseX;
    private int lastMouseY;

    public IrcChatScreen(IrcChat module) {
        super(Text.literal("IRC Chat"));
        this.module = module;
    }

    @Override
    public void render(DrawContext context, int mouseX, int mouseY, float delta) {
        lastMouseX = mouseX;
        lastMouseY = mouseY;
    }

    public void renderOverlay(DrawContext context, RenderTickCounter tickCounter) {
        if (client == null || client.getWindow() == null) return;

        context.createNewRootLayer();
        float scale = getScale();
        float mx = lastMouseX / scale;
        float my = lastMouseY / scale;
        float screenW = client.getWindow().getWidth() / (float) FIXED_GUI_SCALE;
        float screenH = client.getWindow().getHeight() / (float) FIXED_GUI_SCALE;
        float x = (screenW - PANEL_WIDTH) / 2f;
        float y = (screenH - PANEL_HEIGHT) / 2f;

        Render2D.blur(0, 0, Render2D.getFixedScaledWidth(), Render2D.getFixedScaledHeight(), 8.0f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.BLUR_TINT, 0.76f));
        Render2D.rect(0, 0, 5000, 5000, DpClickGuiStyle.alpha(DpClickGuiStyle.OVERLAY, 0.68f), 0f);

        context.getMatrices().pushMatrix();
        context.getMatrices().scale(scale, scale);
        renderGlow(screenW, screenH);
        renderPanel(context, x, y, mx, my);
        context.getMatrices().popMatrix();
    }

    private void renderPanel(DrawContext context, float x, float y, float mouseX, float mouseY) {
        Render2D.blur(x, y, PANEL_WIDTH, PANEL_HEIGHT, 9.0f, 7.0f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.BLUR_TINT, 0.82f));
        Render2D.gradientRect9(x, y, PANEL_WIDTH, PANEL_HEIGHT, DpClickGuiStyle.hudGradient(1f), 7.0f);
        Render2D.outline(x, y, PANEL_WIDTH, PANEL_HEIGHT, 0.55f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.STROKE, 0.78f), 7.0f);

        renderHeader(x, y);
        renderMessages(context, x, y);
        renderInput(x, y, mouseX, mouseY);
    }

    private void renderHeader(float x, float y) {
        Render2D.rect(x, y, PANEL_WIDTH, HEADER_HEIGHT, DpClickGuiStyle.PANEL_HEADER, 7f, 7f, 0f, 0f);
        Render2D.rect(x + 16f, y + HEADER_HEIGHT - 1f, PANEL_WIDTH - 32f, 0.55f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.STROKE, 0.72f), 0.3f);

        float iconX = x + PADDING + 2f;
        float iconY = y + (HEADER_HEIGHT - ICON_SIZE) / 2f;
        Render2D.rect(iconX - 4f, iconY - 4f, ICON_SIZE + 8f, ICON_SIZE + 8f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.ROW_ACTIVE, 0.56f), 5f);
        Render2D.texture(CHAT_ICON, iconX, iconY, ICON_SIZE, ICON_SIZE, DpClickGuiStyle.ACCENT_SOFT);

        Fonts.BOLD.draw("IRC Chat", iconX + ICON_SIZE + 7f, y + 8.1f, TITLE_SIZE, DpClickGuiStyle.TEXT);
        Fonts.BOLD.draw(IrcDefaults.DISPLAY_NAME, iconX + ICON_SIZE + 7f, y + 18.3f,
                META_SIZE, DpClickGuiStyle.DIM);

        String status = module.isConnected() ? "online" : (module.isConnecting() ? "connecting" : "offline");
        int statusColor = module.isConnected()
                ? DpClickGuiStyle.SUCCESS
                : (module.isConnecting() ? DpClickGuiStyle.ACCENT_SOFT : DpClickGuiStyle.DANGER);
        float statusW = Fonts.BOLD.getWidth(status, META_SIZE);
        float pillW = statusW + 18f;
        float pillX = x + PANEL_WIDTH - PADDING - pillW;
        float pillY = y + 8.2f;
        Render2D.rect(pillX, pillY, pillW, 12.4f,
                DpClickGuiStyle.alpha(module.isConnected() ? DpClickGuiStyle.ROW_ACTIVE : DpClickGuiStyle.ROW_HOVER, 0.72f),
                4.4f);
        Render2D.rect(pillX + 5f, pillY + 4.5f, 3.4f, 3.4f, statusColor, 1.7f);
        Fonts.BOLD.draw(status, pillX + 11.2f, pillY + 3.6f, META_SIZE, statusColor);
    }

    private void renderMessages(DrawContext context, float x, float y) {
        List<IrcMessage> messages = module.getHistorySnapshot();
        float top = y + HEADER_HEIGHT + PADDING;
        float bottom = y + PANEL_HEIGHT - INPUT_HEIGHT - PADDING - 4f;
        float areaHeight = bottom - top;
        int visibleLines = Math.max(1, (int) (areaHeight / LINE_HEIGHT));
        int maxOffset = Math.max(0, messages.size() - visibleLines);
        scrollOffset = Math.max(0, Math.min(scrollOffset, maxOffset));

        Render2D.rect(x + PADDING, top - 2f, PANEL_WIDTH - PADDING * 2f, areaHeight + 4f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.SETTINGS_BG, 0.45f), 4f);

        Scissor.enable(x + PADDING, top, PANEL_WIDTH - PADDING * 2f, areaHeight, FIXED_GUI_SCALE);
        if (messages.isEmpty()) {
            renderEmptyState(x, top, PANEL_WIDTH - PADDING * 2f, areaHeight);
        } else {
            int start = Math.max(0, messages.size() - visibleLines - scrollOffset);
            int end = Math.min(messages.size(), start + visibleLines);
            float lineY = top + 2.5f;
            for (int i = start; i < end; i++) {
                renderMessage(messages.get(i), x + PADDING + 5f, lineY, PANEL_WIDTH - PADDING * 2f - 10f);
                lineY += LINE_HEIGHT;
            }
        }
        Scissor.disable();

        if (maxOffset > 0) {
            float trackH = areaHeight - 10f;
            float thumbH = Math.max(12f, trackH * visibleLines / Math.max(messages.size(), visibleLines));
            float thumbY = top + 5f + (trackH - thumbH) * (1f - (scrollOffset / (float) maxOffset));
            Render2D.rect(x + PANEL_WIDTH - PADDING - 3f, thumbY, 1.2f, thumbH,
                    DpClickGuiStyle.alpha(DpClickGuiStyle.ACCENT_SOFT, 0.55f), 0.6f);
        }
    }

    private void renderMessage(IrcMessage message, float x, float y, float width) {
        if (message.kind() == IrcMessage.Kind.CHAT) {
            Render2D.rect(x - 3f, y - 2.1f, width + 6f, 12.7f,
                    DpClickGuiStyle.alpha(DpClickGuiStyle.ROW_BG, 0.56f), 2.4f);
            String nick = trimToWidth(message.nickname(), width * 0.32f, NICK_SIZE);
            float nickW = Fonts.BOLD.getWidth(nick, NICK_SIZE);
            Fonts.BOLD.draw(nick, x, y + 1.7f, NICK_SIZE, DpClickGuiStyle.ACCENT_SOFT);
            Render2D.rect(x + nickW + 4f, y + 3.4f, 0.5f, 5.2f,
                    DpClickGuiStyle.alpha(DpClickGuiStyle.STROKE, 0.75f), 0.25f);
            String text = trimToWidth(message.text(), width - nickW - 11f, MESSAGE_SIZE);
            Fonts.BOLD.draw(text, x + nickW + 9f, y + 1.7f, MESSAGE_SIZE, DpClickGuiStyle.MUTED);
            return;
        }

        Render2D.rect(x - 3f, y - 2.1f, width + 6f, 12.7f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.SETTINGS_BG, 0.42f), 2.4f);
        Render2D.rect(x, y + 3.8f, 3.2f, 3.2f, DpClickGuiStyle.alpha(DpClickGuiStyle.DIM, 0.82f), 1.6f);
        Fonts.BOLD.draw(trimToWidth(message.text(), width - 10f, MESSAGE_SIZE), x + 8f, y + 1.7f,
                MESSAGE_SIZE, DpClickGuiStyle.DIM);
    }

    private void renderEmptyState(float x, float top, float width, float height) {
        String title = module.isConnected() ? "Waiting for messages" : "Connect module to start chatting";
        String hint = "Press Enter to send. Esc closes this window.";
        float centerY = top + height / 2f;
        Render2D.texture(CHAT_ICON, x + width / 2f - 7f, centerY - 22f, 14f, 14f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.ACCENT_SOFT, 0.72f));
        Fonts.BOLD.drawCentered(title, x + width / 2f, centerY - 3f, 6.0f, DpClickGuiStyle.MUTED);
        Fonts.BOLD.drawCentered(hint, x + width / 2f, centerY + 8f, 5.1f, DpClickGuiStyle.DIM);
    }

    private void renderInput(float x, float y, float mouseX, float mouseY) {
        float inputX = x + PADDING;
        float inputY = y + PANEL_HEIGHT - INPUT_HEIGHT - PADDING;
        float inputW = PANEL_WIDTH - PADDING * 2f;
        boolean hovered = DpClickGuiStyle.hovered(mouseX, mouseY, inputX, inputY, inputW, INPUT_HEIGHT);

        Render2D.rect(inputX, inputY, inputW, INPUT_HEIGHT,
                DpClickGuiStyle.alpha(hovered ? DpClickGuiStyle.ROW_HOVER : DpClickGuiStyle.ROW_BG, 0.86f), 4.2f);
        Render2D.outline(inputX, inputY, inputW, INPUT_HEIGHT, 0.45f,
                DpClickGuiStyle.alpha(hovered ? DpClickGuiStyle.ACCENT : DpClickGuiStyle.STROKE, hovered ? 0.62f : 0.6f),
                4.2f);

        String shown = input.isEmpty() ? "Message..." : input;
        if (!input.isEmpty() && System.currentTimeMillis() % 1000L > 500L) shown += "_";
        int color = input.isEmpty() ? DpClickGuiStyle.DIM : DpClickGuiStyle.TEXT;
        Fonts.BOLD.draw(trimToWidth(shown, inputW - 42f, INPUT_SIZE), inputX + 8f, inputY + 8.5f, INPUT_SIZE, color);

        float sendX = inputX + inputW - 28f;
        float sendY = inputY + 5f;
        Render2D.rect(sendX, sendY, 18f, 14f,
                DpClickGuiStyle.alpha(input.isBlank() ? DpClickGuiStyle.STROKE : DpClickGuiStyle.ROW_ACTIVE, 0.88f),
                3.4f);
        float arrowColorAlpha = input.isBlank() ? 0.55f : 1f;
        Render2D.rect(sendX + 5f, sendY + 6.5f, 7.0f, 1.0f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.ACCENT_SOFT, arrowColorAlpha), 0.5f);
        Render2D.rect(sendX + 10.4f, sendY + 4.7f, 1.0f, 4.6f,
                DpClickGuiStyle.alpha(DpClickGuiStyle.ACCENT_SOFT, arrowColorAlpha), 0.5f);
    }

    private void renderGlow(float screenW, float screenH) {
        float glowW = Math.max(250f, screenW * 0.42f);
        float glowH = Math.max(250f, screenH * 0.94f);
        Render2D.texture(GLOW_TEXTURE, -glowW * 0.48f, screenH * 0.08f, glowW, glowH,
                DpClickGuiStyle.alpha(DpClickGuiStyle.SIDE_GLOW_SOFT, 0.055f));
        Render2D.texture(GLOW_TEXTURE, screenW - glowW * 0.46f, -glowH * 0.2f, glowW, glowH,
                DpClickGuiStyle.alpha(DpClickGuiStyle.SIDE_GLOW, 0.045f));
    }

    @Override
    public boolean keyPressed(KeyInput inputEvent) {
        int key = inputEvent.key();
        if (key == GLFW.GLFW_KEY_ESCAPE) {
            close();
            return true;
        }
        if (key == GLFW.GLFW_KEY_ENTER || key == GLFW.GLFW_KEY_KP_ENTER) {
            module.sendMessage(input);
            input = "";
            scrollOffset = 0;
            return true;
        }
        if (key == GLFW.GLFW_KEY_BACKSPACE) {
            if (!input.isEmpty()) input = input.substring(0, input.length() - 1);
            return true;
        }
        if (isControlDown() && key == GLFW.GLFW_KEY_V && client != null) {
            String clipboard = client.keyboard.getClipboard();
            if (clipboard != null && !clipboard.isEmpty()) appendText(clipboard);
            return true;
        }
        return super.keyPressed(inputEvent);
    }

    @Override
    public boolean charTyped(CharInput charInput) {
        char chr = (char) charInput.codepoint();
        if (!Character.isISOControl(chr)) {
            appendText(String.valueOf(chr));
            return true;
        }
        return super.charTyped(charInput);
    }

    @Override
    public boolean mouseScrolled(double mouseX, double mouseY, double horizontal, double vertical) {
        scrollOffset = Math.max(0, scrollOffset - (int) Math.signum(vertical) * 3);
        return true;
    }

    @Override
    public boolean mouseClicked(Click click, boolean doubled) {
        return true;
    }

    @Override
    public boolean shouldPause() {
        return false;
    }

    private void appendText(String text) {
        String normalized = text.replace('\n', ' ').replace('\r', ' ');
        int remaining = Math.max(0, 200 - input.length());
        if (remaining > 0) {
            input += normalized.substring(0, Math.min(remaining, normalized.length()));
        }
    }

    private boolean isControlDown() {
        long handle = client.getWindow().getHandle();
        return GLFW.glfwGetKey(handle, GLFW.GLFW_KEY_LEFT_CONTROL) == GLFW.GLFW_PRESS
                || GLFW.glfwGetKey(handle, GLFW.GLFW_KEY_RIGHT_CONTROL) == GLFW.GLFW_PRESS;
    }

    private float getScale() {
        int guiScale = client.getWindow().calculateScaleFactor(client.options.getGuiScale().getValue(), client.forcesUnicodeFont());
        return (float) FIXED_GUI_SCALE / guiScale;
    }

    private String trimToWidth(String text, float maxWidth, float size) {
        String value = text == null ? "" : text;
        if (Fonts.BOLD.getWidth(value, size) <= maxWidth) return value;
        String suffix = "...";
        int end = value.length();
        while (end > 0 && Fonts.BOLD.getWidth(value.substring(0, end) + suffix, size) > maxWidth) {
            end--;
        }
        return value.substring(0, Math.max(0, end)) + suffix;
    }
}
