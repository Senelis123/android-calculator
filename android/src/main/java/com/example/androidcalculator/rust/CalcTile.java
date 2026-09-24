package com.example.androidcalculator.rust;

import android.app.PendingIntent;
import android.content.Intent;
import android.os.Build;
import android.service.quicksettings.TileService;

/** Quick settings tile: opens the calculator. */
public class CalcTile extends TileService {
    @Override public void onClick() {
        Intent i = new Intent(this, android.app.NativeActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        if (Build.VERSION.SDK_INT >= 34) startActivityAndCollapse(PendingIntent.getActivity(this, 0, i, PendingIntent.FLAG_IMMUTABLE));
        else startActivityAndCollapse(i);
    }
}
