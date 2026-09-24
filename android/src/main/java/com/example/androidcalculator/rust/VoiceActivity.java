package com.example.androidcalculator.rust;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.speech.RecognizerIntent;
import java.util.ArrayList;

/** Invisible activity: opens the phone's speech recognizer and hands the text back to the app. */
public class VoiceActivity extends Activity {
    @Override protected void onCreate(Bundle b) {
        super.onCreate(b);
        Intent i = new Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH);
        i.putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);
        String lang = getIntent().getStringExtra("lang");
        if (lang != null) i.putExtra(RecognizerIntent.EXTRA_LANGUAGE, lang);
        try { startActivityForResult(i, 1); } catch (Throwable t) { Addon.voiceResult = ""; finish(); }
    }
    @Override protected void onActivityResult(int req, int res, Intent data) {
        String text = "";
        if (res == RESULT_OK && data != null) {
            ArrayList<String> r = data.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS);
            if (r != null && !r.isEmpty()) text = r.get(0);
        }
        Addon.voiceResult = text;
        finish();
    }
}
