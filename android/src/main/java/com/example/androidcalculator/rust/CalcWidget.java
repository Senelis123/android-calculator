package com.example.androidcalculator.rust;

import android.app.PendingIntent;
import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.Context;
import android.content.Intent;
import android.widget.RemoteViews;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import org.json.JSONArray;
import org.json.JSONObject;

/** Home screen widget: shows the last result; tap opens the calculator. */
public class CalcWidget extends AppWidgetProvider {
    static String lastResult(Context c) {
        try {
            File f = new File(c.getFilesDir(), "saved.json");
            JSONObject o = new JSONObject(new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8));
            JSONArray h = o.optJSONArray("history");
            if (h != null && h.length() > 0) {
                JSONObject e = h.getJSONObject(0);
                return e.optString("result", "0");
            }
        } catch (Throwable ignored) {}
        return "0";
    }
    @Override public void onUpdate(Context c, AppWidgetManager m, int[] ids) {
        for (int id : ids) {
            RemoteViews v = new RemoteViews(c.getPackageName(), R.layout.widget);
            v.setTextViewText(R.id.widget_result, lastResult(c));
            Intent i = new Intent(c, android.app.NativeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            v.setOnClickPendingIntent(R.id.widget_root, PendingIntent.getActivity(c, 0, i, PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT));
            m.updateAppWidget(id, v);
        }
    }
}
