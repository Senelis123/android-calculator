package com.example.androidcalculator.rust;

import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.net.Uri;
import android.os.Build;
import android.os.IBinder;
import android.provider.Settings;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.GridLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Small calculator window that floats over other apps. Uses the app's Rust engine for results. */
public class FloatingService extends Service {
    private WindowManager wm;
    private View root;
    private final StringBuilder expr = new StringBuilder();

    static String start(Context c) {
        if (Build.VERSION.SDK_INT >= 23 && !Settings.canDrawOverlays(c)) {
            Intent i = new Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + c.getPackageName())).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            try { c.startActivity(i); } catch (Throwable ignored) {}
            return "permission";
        }
        c.startService(new Intent(c, FloatingService.class));
        return "ok";
    }

    @Override public IBinder onBind(Intent i) { return null; }

    @Override public void onCreate() {
        super.onCreate();
        wm = (WindowManager) getSystemService(WINDOW_SERVICE);
        float d = getResources().getDisplayMetrics().density;
        LinearLayout box = new LinearLayout(this);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setBackgroundColor(Color.argb(240, 20, 20, 22));
        box.setPadding((int) (6 * d), (int) (6 * d), (int) (6 * d), (int) (6 * d));
        final TextView disp = new TextView(this);
        disp.setTextColor(Color.WHITE); disp.setTextSize(22); disp.setGravity(Gravity.END); disp.setText("0");
        disp.setContentDescription("Result");
        box.addView(disp);
        GridLayout g = new GridLayout(this);
        g.setColumnCount(4);
        String[] keys = {"7", "8", "9", "/", "4", "5", "6", "*", "1", "2", "3", "-", "0", ".", "=", "+", "C", "(", ")", "×"};
        for (final String k : keys) {
            Button b = new Button(this);
            b.setText(k.equals("×") ? "✕" : k);
            b.setContentDescription(k.equals("×") ? "Close" : k);
            b.setMinWidth(0); b.setMinimumWidth(0);
            GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
            lp.width = (int) (52 * d); lp.height = (int) (48 * d);
            b.setLayoutParams(lp);
            b.setOnClickListener(v -> {
                switch (k) {
                    case "×": stopSelf(); return;
                    case "C": expr.setLength(0); disp.setText("0"); return;
                    case "=": {
                        String r = Addon.nativeEval(expr.toString());
                        disp.setText(r); disp.announceForAccessibility(r);
                        expr.setLength(0); if (!r.equals("Error")) expr.append(r.replace(',', '.').replace(" ", "").replace("\u202F", ""));
                        return;
                    }
                    default: expr.append(k); disp.setText(expr.toString());
                }
            });
            g.addView(b);
        }
        box.addView(g);
        root = box;
        int type = Build.VERSION.SDK_INT >= 26 ? WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY : WindowManager.LayoutParams.TYPE_PHONE;
        final WindowManager.LayoutParams p = new WindowManager.LayoutParams(WindowManager.LayoutParams.WRAP_CONTENT, WindowManager.LayoutParams.WRAP_CONTENT, type,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE, PixelFormat.TRANSLUCENT);
        p.gravity = Gravity.TOP | Gravity.START; p.x = (int) (20 * d); p.y = (int) (120 * d);
        disp.setOnTouchListener(new View.OnTouchListener() {
            float sx, sy; int px, py;
            @Override public boolean onTouch(View v, MotionEvent e) {
                if (e.getAction() == MotionEvent.ACTION_DOWN) { sx = e.getRawX(); sy = e.getRawY(); px = p.x; py = p.y; return true; }
                if (e.getAction() == MotionEvent.ACTION_MOVE) { p.x = px + (int) (e.getRawX() - sx); p.y = py + (int) (e.getRawY() - sy); wm.updateViewLayout(root, p); return true; }
                return false;
            }
        });
        wm.addView(root, p);
    }

    @Override public void onDestroy() { if (root != null) wm.removeView(root); super.onDestroy(); }
}
