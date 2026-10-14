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
    private static volatile String dialogValue;
    public static String inputDialog(Context c, String arg) {
        if (!(c instanceof Activity)) return null;
        final Activity a=(Activity)c;
        try {
            final org.json.JSONObject obj=new org.json.JSONObject(arg);
            a.runOnUiThread(() -> {
                try {
                    final android.widget.EditText input=new android.widget.EditText(a);
                    input.setText(obj.optString("text"));input.setSingleLine(false);
                    input.setInputType(android.text.InputType.TYPE_CLASS_TEXT|android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE|android.text.InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
                    input.setMinLines(3);input.setMaxLines(8);input.setSelectAllOnFocus(true);
                    input.setFilters(new android.text.InputFilter[]{new android.text.InputFilter.LengthFilter(2000)});
                    new android.app.AlertDialog.Builder(a).setTitle(obj.optString("title")).setView(input)
                        .setNegativeButton("Cancel",null).setPositiveButton("OK",(dialog,which)-> {
                            try {dialogValue=new org.json.JSONObject().put("kind",obj.optInt("kind")).put("text",input.getText().toString()).toString();}catch(Exception ignored){}
                        }).show();input.requestFocus();
                }catch(Throwable ignored){}
            });return "ok";
        }catch(Exception ex){return null;}
    }
    public static String dialogResult(Context c,String unused){String result=dialogValue;dialogValue=null;return result;}
    public static String shareImage(Context c,String arg) {
        if (!(c instanceof Activity)) return null;
        final Activity a=(Activity)c;
        try {
            final org.json.JSONObject obj=new org.json.JSONObject(arg);
            a.runOnUiThread(()-> {
                try {
                    final int width=1080;
                    android.text.TextPaint title=new android.text.TextPaint(android.graphics.Paint.ANTI_ALIAS_FLAG);title.setColor(0xff667085);title.setTextSize(36);
                    android.text.TextPaint expression=new android.text.TextPaint(android.graphics.Paint.ANTI_ALIAS_FLAG);expression.setColor(0xff202938);expression.setTextSize(54);
                    android.text.TextPaint result=new android.text.TextPaint(android.graphics.Paint.ANTI_ALIAS_FLAG);result.setColor(0xff315ee8);result.setTextSize(76);result.setTypeface(android.graphics.Typeface.DEFAULT_BOLD);
                    android.text.StaticLayout l=android.text.StaticLayout.Builder.obtain(obj.optString("line"),0,obj.optString("line").length(),expression,936).setIncludePad(false).build();
                    String value="= "+obj.optString("result");
                    android.text.StaticLayout r=android.text.StaticLayout.Builder.obtain(value,0,value.length(),result,936).setIncludePad(false).build();
                    int height=200+l.getHeight()+r.getHeight()+120;
                    android.graphics.Bitmap bitmap=android.graphics.Bitmap.createBitmap(width,height,android.graphics.Bitmap.Config.ARGB_8888);
                    android.graphics.Canvas canvas=new android.graphics.Canvas(bitmap);canvas.drawColor(0xfff7f9ff);canvas.drawText("Calculator",72,80,title);
                    canvas.save();canvas.translate(72,140);l.draw(canvas);canvas.restore();
                    canvas.save();canvas.translate(72,180+l.getHeight());r.draw(canvas);canvas.restore();
                    java.io.File folder=new java.io.File(c.getCacheDir(),"shared");folder.mkdirs();
                    java.io.File file=new java.io.File(folder,"calculation.png");
                    try(java.io.FileOutputStream stream=new java.io.FileOutputStream(file)){bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG,100,stream);}bitmap.recycle();
                    android.net.Uri uri=androidx.core.content.FileProvider.getUriForFile(c,c.getPackageName()+".files",file);
                    Intent intent=new Intent(Intent.ACTION_SEND).setType("image/png").putExtra(Intent.EXTRA_STREAM,uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                    intent.setClipData(android.content.ClipData.newRawUri("Calculation",uri));a.startActivity(Intent.createChooser(intent,"Share calculation"));
                }catch(Throwable ex){android.widget.Toast.makeText(a,"Could not share image",android.widget.Toast.LENGTH_LONG).show();}
            });return "ok";
        }catch(Exception ex){return null;}
    }
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

    /**
     * Makes the window edge-to-edge (Android 11+) and returns the system bar / cutout insets in pixels as
     * "left,top,right,bottom". The Rust UI keeps its content clear of them. arg = "dark" or "light" theme.
     */
    public static String insets(Context c, String arg) {
        if (Build.VERSION.SDK_INT < 30 || !(c instanceof Activity)) return "0,0,0,0";
        final Activity a = (Activity) c;
        final boolean dark = "dark".equals(arg);
        final String[] res = { "0,0,0,0" };
        final java.util.concurrent.CountDownLatch done = new java.util.concurrent.CountDownLatch(1);
        a.runOnUiThread(() -> {
            try {
                android.view.Window w = a.getWindow();
                w.setDecorFitsSystemWindows(false);
                w.setStatusBarColor(0);
                w.setNavigationBarColor(0);
                if (Build.VERSION.SDK_INT >= 29) { w.setNavigationBarContrastEnforced(false); w.setStatusBarContrastEnforced(false); }
                android.view.WindowInsetsController ic = w.getInsetsController();
                if (ic != null) {
                    int m = android.view.WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS | android.view.WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                    ic.setSystemBarsAppearance(dark ? 0 : m, m);
                }
                android.view.WindowInsets wi = w.getDecorView().getRootWindowInsets();
                int l = 0, t = 0, r = 0, b = 0;
                if (wi != null) {
                    android.graphics.Insets in = wi.getInsets(android.view.WindowInsets.Type.systemBars() | android.view.WindowInsets.Type.displayCutout());
                    l = in.left; t = in.top; r = in.right; b = in.bottom;
                }
                if (t == 0) {
                    android.content.res.Resources rs = a.getResources();
                    int id = rs.getIdentifier("status_bar_height", "dimen", "android");
                    if (id > 0) t = rs.getDimensionPixelSize(id);
                }
                res[0] = l + "," + t + "," + r + "," + b;
            } catch (Throwable ignored) {
            } finally { done.countDown(); }
        });
        try { done.await(500, TimeUnit.MILLISECONDS); } catch (InterruptedException ignored) {}
        return res[0];
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
