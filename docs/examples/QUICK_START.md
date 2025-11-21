# Quick Start: CPL with Avatar

## Steps

1. **Set up provider API key** (example for Beyond Presence):
   ```bash
   export BEYOND_PRESENCE_API_KEY="your-api-key-here"
   ```

2. **Start the server**:
   ```bash
   cargo run
   ```

3. **Start the UI** (in another terminal):
   ```bash
   cd narayana-ui && npm run dev
   ```

4. **Create a CPL with avatar**:
   - Navigate to http://localhost:5173/cpls
   - Click "Create CPL"
   - Scroll to "Avatar (Virtual 3D Interface)" section
   - Check "Enable Avatar"
   - Configure settings:
     - Provider: Beyond Presence (or your preferred provider)
     - Expression Sensitivity: 0.7
     - Animation Speed: 1.0
     - Enable Lip Sync: ✓
     - Enable Gestures: ✓
   - Click "Create CPL"

5. **Start the CPL**:
   - Find your CPL in the list
   - Click "Start" button
   - Wait for CPL to be running

6. **Open Avatar Window**:
   - Once CPL is running, an "Avatar" button appears
   - Click "Avatar" button
   - Avatar window opens in new tab/window

7. **Interact with Avatar**:
   - Avatar displays real-time expressions and gestures from CPL
   - WebSocket connection status shown in top-right
   - Avatar state information shown at bottom

## Troubleshooting

- **Avatar button not showing**: Ensure CPL is running and avatar is enabled
- **Connection issues**: Check AvatarBridge is running on port 8081
- **No avatar rendering**: Verify provider API key is set correctly
- **WebSocket errors**: Check browser console for connection errors

## API Example

```bash
curl -X POST http://localhost:8080/api/v1/cpls \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "config": {
      "enable_avatar": true,
      "avatar_config": {
        "enabled": true,
        "provider": "BeyondPresence",
        "expression_sensitivity": 0.7,
        "animation_speed": 1.0,
        "enable_lip_sync": true,
        "enable_gestures": true
      }
    }
  }'
```
