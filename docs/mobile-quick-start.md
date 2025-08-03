# Mobile SMS/MMS Quick Start Guide

> Get up and running with SMS/MMS in Comunicado in 5 minutes
> Version: 1.0.0

## Quick Setup Checklist

### 1. Install KDE Connect (2 minutes)

```bash
# Ubuntu/Debian
sudo apt install kdeconnect

# Fedora  
sudo dnf install kdeconnect

# Arch Linux
sudo pacman -S kdeconnect
```

### 2. Install Mobile App (1 minute)

- Download **KDE Connect** from [Google Play Store](https://play.google.com/store/apps/details?id=org.kde.kdeconnect_tp)
- Grant SMS and notification permissions

### 3. Pair Devices (2 minutes)

```bash
# Start KDE Connect
kdeconnect-cli --refresh

# List available devices
kdeconnect-cli --list-available

# Pair with your phone (replace DEVICE_ID)
kdeconnect-cli --pair --device [DEVICE_ID]
```

Accept pairing request on your phone.

### 4. Test SMS

```bash
# Test sending SMS (replace with real number)
kdeconnect-cli --device [DEVICE_ID] --send-sms "Test from Comunicado" --destination "+1234567890"
```

### 5. Use in Comunicado

1. Start Comunicado: `comunicado`
2. Open mobile interface: `Ctrl+M`
3. Compose message: `C`
4. Enter phone number and message
5. Send: `Ctrl+Enter`

## Key Shortcuts

- `Ctrl+M` - Open mobile interface
- `C` - Compose new message
- `R` - Refresh messages
- `Enter` - Open conversation
- `Esc` - Go back/close
- `/` - Search messages

## Troubleshooting

**Device not found?**
```bash
# Restart KDE Connect
systemctl --user restart kdeconnect
kdeconnect-cli --refresh
```

**SMS not working?**
- Check SMS permissions in KDE Connect app on phone
- Verify both devices on same WiFi network
- Try unpairing and re-pairing devices

**Need help?** See full documentation:
- [User Manual](sms-mms-user-manual.md)
- [Technical Guide](kde-connect-plugin-guide.md)

---

That's it! You should now be able to send and receive SMS messages directly from Comunicado. For advanced features and troubleshooting, see the full user manual.