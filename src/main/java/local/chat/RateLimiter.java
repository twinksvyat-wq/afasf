package local.chat;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.util.Objects;

public final class RateLimiter {
    private final Duration interval;
    private final Clock clock;
    private Instant nextAllowedAt = Instant.MIN;

    public RateLimiter(Duration interval, Clock clock) {
        if (interval.isNegative() || interval.isZero()) {
            throw new IllegalArgumentException("interval must be positive");
        }
        this.interval = interval;
        this.clock = Objects.requireNonNull(clock, "clock");
    }

    public synchronized boolean tryAcquire() {
        Instant now = clock.instant();
        if (now.isBefore(nextAllowedAt)) {
            return false;
        }
        nextAllowedAt = now.plus(interval);
        return true;
    }
}
