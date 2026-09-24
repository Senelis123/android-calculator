package com.example.androidcalculator.rust;

import com.google.mlkit.genai.prompt.Generation;
import com.google.mlkit.genai.prompt.GenerateContentResponse;
import com.google.mlkit.genai.prompt.java.GenerativeModelFutures;
import java.util.concurrent.TimeUnit;

/** Gemini Nano through ML Kit GenAI Prompt API. Everything runs on the phone. */
final class Nano {
    private static GenerativeModelFutures model;
    private static synchronized GenerativeModelFutures model() {
        if (model == null) model = GenerativeModelFutures.from(Generation.INSTANCE.getClient());
        return model;
    }
    static String call(android.content.Context c, String arg) throws Exception {
        // ML Kit is started here (not at app start) so a problem in it cannot stop the app from opening
        com.google.mlkit.common.sdkinternal.MlKitContext.initializeIfNeeded(c.getApplicationContext());
        if (arg.equals("status")) {
            int s = model().checkStatus().get(10, TimeUnit.SECONDS);
            // FeatureStatus: 0 unavailable, 1 downloadable, 2 downloading, 3 available
            if (s == 1) { try { model().download(new com.google.mlkit.genai.common.DownloadCallback() {}); } catch (Throwable ignored) {} }
            return s == 3 ? "available" : s == 1 ? "downloadable" : s == 2 ? "downloading" : "unavailable";
        }
        if (arg.startsWith("ask\t")) {
            GenerateContentResponse r = model().generateContent(arg.substring(4)).get(30, TimeUnit.SECONDS);
            return r.getCandidates().get(0).getText();
        }
        return null;
    }
}
