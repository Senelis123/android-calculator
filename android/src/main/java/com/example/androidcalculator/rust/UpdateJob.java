package com.example.androidcalculator.rust;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.job.JobInfo;
import android.app.job.JobParameters;
import android.app.job.JobScheduler;
import android.app.job.JobService;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Scanner;
import org.json.JSONObject;

/** Checks GitHub for a newer release about once a day and shows a notification. */
public class UpdateJob extends JobService {
    static final int ID = 7001;
    static void schedule(Context c, boolean on) {
        JobScheduler js = (JobScheduler) c.getSystemService(Context.JOB_SCHEDULER_SERVICE);
        if (js == null) return;
        if (!on) { js.cancel(ID); return; }
        js.schedule(new JobInfo.Builder(ID, new ComponentName(c, UpdateJob.class))
            .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY).setPeriodic(24L * 3600 * 1000).setPersisted(false).build());
    }
    static int[] parse(String v) {
        String[] p = v.replaceFirst("^v", "").split("\\.");
        int[] r = new int[4];
        for (int i = 0; i < 4 && i < p.length; i++) { try { r[i] = Integer.parseInt(p[i].replaceAll("[^0-9]", "")); } catch (Exception ignored) {} }
        return r;
    }
    static boolean newer(String a, String b) {
        int[] x = parse(a), y = parse(b);
        for (int i = 0; i < 4; i++) if (x[i] != y[i]) return x[i] > y[i];
        return false;
    }
    @Override public boolean onStartJob(JobParameters p) {
        new Thread(() -> {
            try {
                HttpURLConnection c = (HttpURLConnection) new URL("https://api.github.com/repos/" + BuildConfig.UPDATE_REPO + "/releases/latest").openConnection();
                c.setRequestProperty("Accept", "application/vnd.github+json");
                c.setConnectTimeout(15000); c.setReadTimeout(15000);
                try (InputStream in = c.getInputStream(); Scanner s = new Scanner(in).useDelimiter("\\A")) {
                    String tag = new JSONObject(s.hasNext() ? s.next() : "{}").optString("tag_name", "");
                    if (!tag.isEmpty() && newer(tag, BuildConfig.VERSION_NAME)) notifyUpdate(tag);
                }
            } catch (Throwable ignored) {}
            jobFinished(p, false);
        }).start();
        return true;
    }
    @Override public boolean onStopJob(JobParameters p) { return true; }
    private void notifyUpdate(String tag) {
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (nm == null) return;
        if (Build.VERSION.SDK_INT >= 26) nm.createNotificationChannel(new NotificationChannel("updates", "Updates", NotificationManager.IMPORTANCE_DEFAULT));
        Intent i = new Intent(this, android.app.NativeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        Notification.Builder b = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(this, "updates") : new Notification.Builder(this);
        b.setSmallIcon(R.mipmap.ic_launcher).setContentTitle("Calculator " + tag).setContentText("Update available - tap to open")
            .setAutoCancel(true).setContentIntent(PendingIntent.getActivity(this, 0, i, PendingIntent.FLAG_IMMUTABLE));
        try { nm.notify(1, b.build()); } catch (SecurityException ignored) {}
    }
}
