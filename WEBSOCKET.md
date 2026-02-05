# WebSocket Server Documentation

## Overview

The DC-09 receiver now includes an optional WebSocket server that broadcasts all received alarm messages to connected clients in real-time JSON format.

## Usage

### Enable WebSocket Server

```bash
./receiver --websocket --websocket-port 8081
```

### Command Line Options

| Option              | Description                                    | Default |
|---------------------|------------------------------------------------|---------|
| `--websocket`       | Enable WebSocket server                        | false   |
| `--websocket-port`  | Port for WebSocket server                      | 8081    |

## JSON Message Format

Each alarm is broadcast as a JSON object with the following structure:

```json
{
  "token": "SIA-DCS",
  "sequence": 1,
  "receiver": "R001",
  "line_prefix": "L001",
  "account": "1234",
  "data": "BA|ABurglar Alarm",
  "extended": ["Additional data"],
  "timestamp": "12:34:56,01-15-2024"
}
```

### Field Descriptions

- **token** (string, required): Message ID token (e.g., "NULL", "SIA-DCS", "ADM-CID")
- **sequence** (number, required): Message sequence number
- **receiver** (string, optional): Receiver identifier (e.g., "R001")
- **line_prefix** (string, optional): Line prefix identifier (e.g., "L001")
- **account** (string, required): Account number
- **data** (string, optional): Message payload
- **extended** (array, optional): Extended data fields
- **timestamp** (string, optional): Timestamp in format "HH:MM:SS,MM-DD-YYYY"

Note: Optional fields are omitted from JSON if not present in the alarm.

## Client Examples

### JavaScript/Browser

```javascript
const ws = new WebSocket('ws://127.0.0.1:8081');

ws.onopen = () => {
    console.log('Connected to DC-09 receiver');
};

ws.onmessage = (event) => {
    const alarm = JSON.parse(event.data);
    console.log('Received alarm:', alarm);
};

ws.onerror = (error) => {
    console.error('WebSocket error:', error);
};

ws.onclose = () => {
    console.log('Disconnected from DC-09 receiver');
};
```

### Python

```python
import asyncio
import websockets
import json

async def receive_alarms():
    uri = "ws://127.0.0.1:8081"
    async with websockets.connect(uri) as websocket:
        async for message in websocket:
            alarm = json.loads(message)
            print(f"Received alarm: {alarm}")

asyncio.run(receive_alarms())
```

## Architecture

The WebSocket server uses a broadcast channel to distribute messages to all connected clients:

1. DC-09 messages are received via TCP or UDP
2. After successful processing, alarms are converted to JSON
3. JSON messages are broadcast to all connected WebSocket clients
4. Clients receive messages in real-time

### Performance Characteristics

- **Non-blocking**: WebSocket broadcasting doesn't impact DC-09 message processing
- **Scalable**: Supports multiple concurrent WebSocket clients
- **Efficient**: Uses Tokio's broadcast channel with configurable capacity (100 messages)
- **Zero overhead**: When disabled, no WebSocket code is executed

## Security Considerations

- WebSocket runs on a separate port from the DC-09 receiver
- No authentication is implemented (designed for trusted networks)
- All messages are broadcast to all connected clients
- Client connections are receive-only (no commands accepted)

For production use in untrusted networks, consider:
- Adding TLS/SSL support (wss://)
- Implementing authentication
- Using a reverse proxy with access controls

## Troubleshooting

### Connection Refused
- Ensure `--websocket` flag is set
- Verify the WebSocket port is not blocked by firewall
- Check if another service is using the port

### No Messages Received
- Verify DC-09 messages are being received by the receiver
- Check receiver logs for errors
- Ensure client is connected before messages are sent

### Messages Lost
- If clients can't keep up, older messages may be dropped
- Increase `BROADCAST_CHANNEL_CAPACITY` in websocket.rs if needed
- Ensure client processes messages quickly
