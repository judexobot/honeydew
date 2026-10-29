package dev.daijro.honeydew;

import android.content.Context;
import android.media.midi.MidiDevice;
import android.media.midi.MidiDeviceInfo;
import android.media.midi.MidiManager;
import android.media.midi.MidiOutputPort;
import android.media.midi.MidiReceiver;
import android.os.Handler;
import android.os.Looper;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

/** USB MIDI service adapter. Audio rendering never calls Java or takes these locks. */
final class HoneydewMidiBridge {
    private static native void onMidiMessage(byte[] message, int count);
    private final MidiManager manager;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final List<MidiDevice> devices = new ArrayList<>();
    private final List<MidiOutputPort> ports = new ArrayList<>();
    private int generation;
    HoneydewMidiBridge(Context context) {
        manager = (MidiManager) context.getSystemService(Context.MIDI_SERVICE);
        if (manager != null) manager.registerDeviceCallback(new MidiManager.DeviceCallback() {
            @Override public void onDeviceRemoved(MidiDeviceInfo device) {
                // An unplug cannot deliver Note Off. Send All Sound Off on every channel.
                byte[] panic = new byte[] {(byte) 0xB0, 120, 0};
                for (int ch = 0; ch < 16; ch++) { panic[0] = (byte) (0xB0 | ch); onMidiMessage(panic, 3); }
            }
        }, handler);
    }
    private static String label(MidiDeviceInfo device, MidiDeviceInfo.PortInfo port) {
        String name = device.getProperties().getString(MidiDeviceInfo.PROPERTY_NAME, "USB MIDI");
        String portName = port.getName();
        return name + " / " + (portName.isEmpty() ? "Port " + (port.getPortNumber() + 1) : portName);
    }
    String[] list() {
        List<String> names = new ArrayList<>();
        if (manager != null) for (MidiDeviceInfo device : manager.getDevices()) {
            if (device.getType() != MidiDeviceInfo.TYPE_USB) continue;
            for (MidiDeviceInfo.PortInfo port : device.getPorts())
                if (port.getType() == MidiDeviceInfo.PortInfo.TYPE_OUTPUT) names.add(label(device, port));
        }
        return names.toArray(new String[0]);
    }
    void open(String choice) {
        handler.post(() -> {
            closeNow();
            final int requested = generation;
            if (manager == null || "<no midi input>".equals(choice)) return;
            for (MidiDeviceInfo info : manager.getDevices()) {
                if (info.getType() != MidiDeviceInfo.TYPE_USB) continue;
                manager.openDevice(info, device -> {
                    if (device == null) return;
                    if (requested != generation) { try { device.close(); } catch (IOException ignored) {} return; }
                    boolean used = false;
                    for (MidiDeviceInfo.PortInfo portInfo : info.getPorts()) {
                        if (portInfo.getType() != MidiDeviceInfo.PortInfo.TYPE_OUTPUT) continue;
                        if (!"Any available".equals(choice) && !choice.equals(label(info, portInfo))) continue;
                        MidiOutputPort port = device.openOutputPort(portInfo.getPortNumber());
                        if (port == null) continue;
                        port.connect(new Receiver()); ports.add(port); used = true;
                    }
                    if (used) devices.add(device);
                    else try { device.close(); } catch (IOException ignored) {}
                }, handler);
            }
        });
    }
    void close() { handler.post(this::closeNow); }
    private void closeNow() {
        generation++;
        for (MidiOutputPort port : ports) try { port.close(); } catch (IOException ignored) {}
        for (MidiDevice device : devices) try { device.close(); } catch (IOException ignored) {}
        ports.clear(); devices.clear();
    }
    /** Running status, fragmented packets and interleaved real-time bytes. */
    private static final class Receiver extends MidiReceiver {
        private final byte[] message = new byte[3];
        private int status, count, needed;
        @Override public void onSend(byte[] bytes, int offset, int length, long timestamp) {
            for (int i = offset; i < offset + length; i++) {
                int value = bytes[i] & 255;
                if (value >= 0xF8) continue;
                if ((value & 0x80) != 0) {
                    count = 0;
                    if (value >= 0xF0) { status = 0; continue; }
                    status = value; needed = (value >> 4 == 0xC || value >> 4 == 0xD) ? 1 : 2;
                } else if (status != 0) {
                    message[0] = (byte) status; message[++count] = (byte) value;
                    if (count == needed) { onMidiMessage(message, needed + 1); count = 0; }
                }
            }
        }
        @Override public void onFlush() {
            status = 0; count = 0;
        }
    }
}
