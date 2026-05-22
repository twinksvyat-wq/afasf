# Java TCP Chat Server

Line-based TCP chat server for Java 21.

## Requirements

- Java 21

Gradle does not need to be installed globally. Use the included Gradle Wrapper.

## Build

```powershell
.\gradlew.bat clean test runnableJar
```

The runnable JAR is created in the project root:

```text
chat-server.jar
```

## Run

Run with the default port `20036`:

```powershell
java -jar chat-server.jar
```

Run with an explicit port:

```powershell
java -jar chat-server.jar 20036
```

## Protocol

Each message is one UTF-8 line ending with `\n`.

Client commands:

```text
HELLO|nickname
MSG|text
PING
QUIT
```

Server messages:

```text
SYSTEM|Connected
CHAT|nickname|text
SYSTEM|nickname disconnected
PONG
```

Rules:

- `MSG|text` is limited to 200 characters.
- Each client can send at most one `MSG` per second.
- The server accepts up to 100 concurrent clients.
- No database is used.
