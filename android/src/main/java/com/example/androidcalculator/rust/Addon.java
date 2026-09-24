package com.example.androidcalculator.rust;

import android.app.Activity;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.speech.tts.TextToSpeech;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

/** Small helpers the Rust app calls through JNI. Every method returns null when a feature is unavailable. */
public final class Addon {
    private static TextToSpeech tts;
    static volatile String voiceResult = null;

    static { try { System.loadLibrary("calculator"); } catch (Throwable ignored) {} }
    public static native String nativeEval(String expr);

    public static String speak(Context c, String arg) {
        final String[] p = arg.split("\t", 2);
        final String text = p.length > 1 ? p[1] : arg;
        final Locale loc = p.length > 1 ? new Locale(p[0]) : Locale.getDefault();
        if (tts == null) {
            final Context app = c.getApplicationContext();
            tts = new TextToSpeech(app, status -> { if (status == TextToSpeech.SUCCESS) { tts.setLanguage(loc); tts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "calc"); } });
        } else { tts.setLanguage(loc); tts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "calc"); }
        announce(c, text);
        return "ok";
    }

    /** TalkBack: reads the text when a screen reader is on. */
    public static String announce(Context c, String text) {
        if (!(c instanceof Activity)) return null;
        final Activity a = (Activity) c;
        a.runOnUiThread(() -> { try { a.getWindow().getDecorView().announceForAccessibility(text); } catch (Throwable ignored) {} });
        return "ok";
    }

    public static String listen(Context c, String lang) {
        voiceResult = null;
        Intent i = new Intent(c, VoiceActivity.class).putExtra("lang", lang);
        if (!(c instanceof Activity)) i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try { c.startActivity(i); return "ok"; } catch (Throwable t) { return null; }
    }

    public static String voiceResult(Context c, String unused) { String r = voiceResult; voiceResult = null; return r; }

    /** Switches between the launcher icons (activity aliases Icon0..Icon2). */
    public static String setIcon(Context c, String which) {
        PackageManager pm = c.getPackageManager();
        for (int i = 0; i < 3; i++) {
            ComponentName cn = new ComponentName(c.getPackageName(), "com.example.androidcalculator.rust.Icon" + i);
            pm.setComponentEnabledSetting(cn, String.valueOf(i).equals(which) ? PackageManager.COMPONENT_ENABLED_STATE_ENABLED : PackageManager.COMPONENT_ENABLED_STATE_DISABLED, PackageManager.DONT_KILL_APP);
        }
        return "ok";
    }

    /** Material You accent colour as #RRGGBB (Android 12+). */
    public static String accent(Context c, String unused) {
        if (Build.VERSION.SDK_INT < 31) return null;
        int col = c.getResources().getColor(android.R.color.system_accent1_500, null);
        return String.format("#%06X", col & 0xFFFFFF);
    }

    public static String fontScale(Context c, String unused) { return String.valueOf(c.getResources().getConfiguration().fontScale); }

    public static String updateChecks(Context c, String on) { UpdateJob.schedule(c, "1".equals(on)); return "ok"; }

    /** Crash info for the "the app closed because of an error" screen: system record since the last check. */
    public static String lastCrash(Context c, String unused) { return Crash.exitInfo(c); }

    public static String floating(Context c, String unused) { return FloatingService.start(c); }

    /** Gemini Nano: "status" or "ask\t<prompt>". Runs on a background thread (blocking). */
    public static String nano(Context c, String arg) {
        if (Build.VERSION.SDK_INT < 26) return arg.equals("status") ? "unavailable" : null;
        try { return Nano.call(c, arg); } catch (Throwable t) { return arg.equals("status") ? "unavailable" : null; }
    }
}
