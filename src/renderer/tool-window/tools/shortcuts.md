# Moonlight Streaming Shortcuts Guide

## 🖥️ PC Clients (Windows / macOS / Linux)

### Session Control

- `Ctrl+Alt+Shift+Q` - Quit the streaming session (the game keeps running on the host)
- `Ctrl+Alt+Shift+D` - Minimize the streaming window
- `Ctrl+Alt+Shift+X` - Toggle fullscreen / windowed mode
- `Ctrl+Alt+Shift+S` - Show the performance stats overlay

> 💡 Performance stats are not supported on Steam Link or Raspberry Pi.

### Mouse Control

- `Ctrl+Alt+Shift+Z` - Toggle mouse and keyboard capture
- `Ctrl+Alt+Shift+M` - Toggle mouse mode (pointer capture / direct control)
- `Ctrl+Alt+Shift+C` - Toggle local cursor visibility in remote-desktop mode
- `Ctrl+Alt+Shift+L` - Lock the mouse pointer to the video area

> 💡 The mouse-lock feature requires enabling the "Optimize mouse for remote desktop" option.

### Other

- `Ctrl+Alt+Shift+V` - Type clipboard text on the host

---

## 📱 Android Client

### Touchpad Mode

- `Single-finger press-and-drag` - Move the mouse cursor
- `Single-finger tap` - Left-click
- `Single-finger long-press` - Right-click
- `Two-finger vertical drag` - Mouse-wheel scroll
- `Three-finger tap` - Show the virtual keyboard

### Touchscreen Mode

- `Tap at a position` - Move the cursor and left-click at that position
- `Long-press at a position` - Right-click at that position
- `Tap and drag` - Click and drag
- `Three-finger tap` - Show the virtual keyboard
- `Two-finger pinch` - Zoom (iOS only)
- `Two-finger drag` - Pan (iOS only)

### Controller Mouse Emulation

- `Long-press Start` - Enable / disable mouse emulation
- `Thumbstick` - Move the mouse cursor
- `A button` - Left-click
- `B button` - Right-click

### Special Combinations (controllers missing certain buttons)

- `R1+Start` - Emulate the Select button
- `Start+Select` - Emulate the Mode button (for controllers with a Select button)

> 💡 Android supports Xbox 360/One and PS3/PS4 controllers, but some controllers connected over Bluetooth may have latency or disconnection issues.

---

## 🍎 iOS / tvOS Client

### iOS Gestures

- `Swipe right from the left edge` - Disconnect the stream
- `Three-finger tap` - Show the virtual keyboard

### tvOS Controls

- `Double-tap Menu` - Disconnect the stream (Apple TV)
- `Apple TV remote touchpad` - Move the mouse and click

### Controller Support

> 💡 iOS 13+ / tvOS 13+ support Xbox One S and PS4 controllers over Bluetooth, including all physical buttons (Select, L3, R3). iOS/tvOS 14+ support controller vibration feedback.

> 📱 iPadOS 13.4+ supports mouse input, with limitations. iPadOS 14+ improves mouse support; for non-Apple mice, prefer USB over Bluetooth for best compatibility.

---

## 📚 Reference

[Official Moonlight Setup Guide](https://github.com/moonlight-stream/moonlight-docs/wiki/Setup-Guide#keyboardmousegamepad-input-options)
