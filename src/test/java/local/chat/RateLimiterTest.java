package local.chat;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import org.junit.jupiter.api.Test;

class RateLimiterTest {
    @Test
    void allowsFirstMessageImmediately() {
        MutableClock clock = new MutableClock(Instant.parse("2026-05-21T00:00:00Z"));
        RateLimiter limiter = new RateLimiter(Duration.ofSeconds(1), clock);

        assertTrue(limiter.tryAcquire());
    }

    @Test
    void rejectsMessagesWithinOneSecond() {
        MutableClock clock = new MutableClock(Instant.parse("2026-05-21T00:00:00Z"));
        RateLimiter limiter = new RateLimiter(Duration.ofSeconds(1), clock);

        assertTrue(limiter.tryAcquire());
        clock.advance(Duration.ofMillis(999));

        assertFalse(limiter.tryAcquire());
    }

    @Test
    void allowsMessagesAfterOneSecond() {
        MutableClock clock = new MutableClock(Instant.parse("2026-05-21T00:00:00Z"));
        RateLimiter limiter = new RateLimiter(Duration.ofSeconds(1), clock);

        assertTrue(limiter.tryAcquire());
        clock.advance(Duration.ofSeconds(1));

        assertTrue(limiter.tryAcquire());
    }

    private static final class MutableClock extends Clock {
        private Instant instant;

        private MutableClock(Instant instant) {
            this.instant = instant;
        }

        private void advance(Duration duration) {
            instant = instant.plus(duration);
        }

        @Override
        public ZoneId getZone() {
            return ZoneId.of("UTC");
        }

        @Override
        public Clock withZone(ZoneId zone) {
            return this;
        }

        @Override
        public Instant instant() {
            return instant;
        }
    }
}
