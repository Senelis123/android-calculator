package com.example.androidcalculator;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Color;
import android.graphics.Typeface;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.GridLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.util.Locale;

public class MainActivity extends Activity {
    private TextView display;
    private String current = "0";
    private double first = Double.NaN;
    private String operator = "";
    private boolean resetOnDigit = false;

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        buildUi();
    }

    private void buildUi() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(12), dp(12), dp(12), dp(12));
        root.setBackgroundColor(Color.WHITE);

        display = new TextView(this);
        display.setText(current);
        display.setTextSize(44);
        display.setTextColor(Color.BLACK);
        display.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        display.setGravity(Gravity.END | Gravity.CENTER_VERTICAL);
        display.setPadding(dp(8), 0, dp(8), 0);
        root.addView(display, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 0, 1.4f));

        GridLayout grid = new GridLayout(this);
        grid.setColumnCount(4);
        String[] keys = {
                "C", "⌫", "÷", "×",
                "7", "8", "9", "−",
                "4", "5", "6", "+",
                "1", "2", "3", "=",
                "0", ".", "±", "%"
        };

        for (String key : keys) {
            Button b = new Button(this);
            b.setText(key);
            b.setTextSize(22);
            b.setAllCaps(false);
            b.setOnClickListener(v -> press(((Button) v).getText().toString()));
            GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
            lp.width = 0;
            lp.height = 0;
            lp.columnSpec = GridLayout.spec(GridLayout.UNDEFINED, 1f);
            lp.rowSpec = GridLayout.spec(GridLayout.UNDEFINED, 1f);
            lp.setMargins(dp(3), dp(3), dp(3), dp(3));
            grid.addView(b, lp);
        }

        root.addView(grid, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 0, 5f));
        setContentView(root);
    }

    private void press(String key) {
        switch (key) {
            case "C":
                current = "0";
                first = Double.NaN;
                operator = "";
                resetOnDigit = false;
                break;
            case "⌫":
                if (!resetOnDigit) {
                    current = current.length() > 1 ? current.substring(0, current.length() - 1) : "0";
                }
                break;
            case ".":
                if (resetOnDigit) {
                    current = "0.";
                    resetOnDigit = false;
                } else if (!current.contains(".")) {
                    current += ".";
                }
                break;
            case "±":
                if (!current.equals("0")) {
                    current = current.startsWith("-") ? current.substring(1) : "-" + current;
                }
                break;
            case "%":
                current = format(parse(current) / 100.0);
                break;
            case "+":
            case "−":
            case "×":
            case "÷":
                selectOperator(key);
                break;
            case "=":
                calculate();
                break;
            default:
                digit(key);
        }
        display.setText(current);
    }

    private void digit(String key) {
        if (resetOnDigit) {
            current = key;
            resetOnDigit = false;
        } else {
            current = current.equals("0") ? key : current + key;
        }
    }

    private void selectOperator(String op) {
        if (!Double.isNaN(first) && !operator.isEmpty() && !resetOnDigit) {
            calculate();
        }
        first = parse(current);
        operator = op;
        resetOnDigit = true;
    }

    private void calculate() {
        if (operator.isEmpty() || Double.isNaN(first)) return;
        double second = parse(current);
        double result;
        switch (operator) {
            case "+": result = first + second; break;
            case "−": result = first - second; break;
            case "×": result = first * second; break;
            case "÷":
                if (second == 0) {
                    current = "Error";
                    first = Double.NaN;
                    operator = "";
                    resetOnDigit = true;
                    return;
                }
                result = first / second;
                break;
            default: return;
        }
        current = format(result);
        first = result;
        operator = "";
        resetOnDigit = true;
    }

    private double parse(String value) {
        try {
            return Double.parseDouble(value);
        } catch (Exception e) {
            return 0.0;
        }
    }

    private String format(double value) {
        if (Double.isNaN(value) || Double.isInfinite(value)) return "Error";
        if (Math.abs(value) < 1e-10) value = 0;
        if (value == Math.rint(value)) return String.format(Locale.US, "%.0f", value);
        return String.format(Locale.US, "%.10f", value)
                .replaceAll("0+$", "")
                .replaceAll("\\.$", "");
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
