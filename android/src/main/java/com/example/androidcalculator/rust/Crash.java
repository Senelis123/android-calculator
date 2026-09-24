package com.example.androidcalculator.rust;

import android.app.ActivityManager;
import android.app.ApplicationExitInfo;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Build;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/** Writes the error of a crash to files/crash.txt so the app can show it on the next start. */
final class Crash {
    private Crash() {}

    static void install(Context c) {
        try {
            final File dir = c.getFilesDir();
            final Thread.UncaughtExceptionHandler prev = Thread.getDefaultUncaughtExceptionHandler();
            Thread.setDefaultUncaughtExceptionHandler((t, e) -> {
                try {
                    StringWriter sw = new StringWriter();
                    e.printStackTrace(new PrintWriter(sw));
                    String s = sw.toString();
                    if (s.length() > 6000) s = s.substring(0, 6000);
                    append(dir, "Java error (" + t.getName() + ", " + BuildConfig.VERSION_NAME + "):\n" + s);
                } catch (Throwable ignored) {}
                if (prev != null) prev.uncaughtException(t, e);
            });
        } catch (Throwable ignored) {}
    }

    static void append(File dir, String text) {
        try (FileOutputStream o = new FileOutputStream(new File(dir, "crash.txt"), true)) {
            o.write((text + "\n\n").getBytes(StandardCharsets.UTF_8));
        } catch (Throwable ignored) {}
    }

    /** Android 11+: the system's record of crashes since the last check (also crashes of older versions). */
    static String exitInfo(Context c) {
        if (Build.VERSION.SDK_INT < 30) return "";
        StringBuilder out = new StringBuilder();
        try {
            SharedPreferences p = c.getSharedPreferences("crash", Context.MODE_PRIVATE);
            long seen = p.getLong("seen", 0), newest = seen;
            ActivityManager am = (ActivityManager) c.getSystemService(Context.ACTIVITY_SERVICE);
            List<ApplicationExitInfo> list = am.getHistoricalProcessExitReasons(c.getPackageName(), 0, 5);
            SimpleDateFormat f = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US);
            for (ApplicationExitInfo i : list) {
                if (i.getTimestamp() <= seen) continue;
                newest = Math.max(newest, i.getTimestamp());
                int r = i.getReason();
                String kind = r == ApplicationExitInfo.REASON_CRASH ? "Java crash" : r == ApplicationExitInfo.REASON_CRASH_NATIVE ? "Native crash"
                    : r == ApplicationExitInfo.REASON_ANR ? "Not responding (ANR)" : r == ApplicationExitInfo.REASON_INITIALIZATION_FAILURE ? "Start failed" : null;
                if (kind == null || seen == 0 && out.length() > 0) continue;
                out.append(kind).append(" at ").append(f.format(new Date(i.getTimestamp())))
                   .append(", status ").append(i.getStatus()).append('\n');
                if (i.getDescription() != null) out.append(i.getDescription()).append('\n');
                if (r == ApplicationExitInfo.REASON_CRASH_NATIVE || r == ApplicationExitInfo.REASON_ANR) out.append(readable(i)).append('\n');
                if (out.length() > 8000) break;
            }
            p.edit().putLong("seen", Math.max(newest, System.currentTimeMillis() - 1000)).apply();
        } catch (Throwable t) { out.append("(exit info: ").append(t).append(")\n"); }
        return out.toString();
    }

    /** Printable pieces of the system trace (the native tombstone is binary; the text parts hold the reason and stack). */
    private static String readable(ApplicationExitInfo i) {
        try (InputStream in = i.getTraceInputStream()) {
            if (in == null) return "";
            ByteArrayOutputStream b = new ByteArrayOutputStream();
            byte[] buf = new byte[8192]; int n, total = 0;
            while ((n = in.read(buf)) > 0 && total < 256 * 1024) { b.write(buf, 0, n); total += n; }
            byte[] d = b.toByteArray();
            StringBuilder s = new StringBuilder(), run = new StringBuilder();
            int lines = 0;
            for (int k = 0; k <= d.length && lines < 60; k++) {
                int ch = k < d.length ? d[k] & 0xFF : 0;
                if (ch >= 0x20 && ch < 0x7F || ch == '\n' || ch == '\t') run.append((char) ch);
                else {
                    String piece = run.toString().trim();
                    if (piece.length() >= 8 && !piece.startsWith("/apex/") && !piece.startsWith("/system/")) { s.append(piece).append('\n'); lines++; }
                    run.setLength(0);
                }
            }
            return s.toString();
        } catch (Throwable t) { return ""; }
    }
}
