package com.example.androidcalculator.rust;

import android.app.Application;
import android.content.Context;

/** Installs the crash catcher as early as possible (before any other app code runs). */
public class App extends Application {
    @Override protected void attachBaseContext(Context base) {
        super.attachBaseContext(base);
        Crash.install(base);
    }
}
