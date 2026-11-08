package com.veloxdb.app

import android.app.AlertDialog
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.util.Log
import androidx.activity.enableEdgeToEdge
import java.io.File
import java.io.PrintWriter
import java.io.StringWriter
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class MainActivity : TauriActivity() {

  companion object {
    private const val TAG = "VeloxCrash"

    fun logCrashToFile(context: android.content.Context?, throwable: Throwable) {
      val sw = StringWriter()
      val pw = PrintWriter(sw)
      throwable.printStackTrace(pw)
      val stackTrace = sw.toString()

      val timestamp = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US).format(Date())
      val report = buildString {
        appendLine("================ VELOXDB CRASH REPORT ================")
        appendLine("Timestamp: $timestamp")
        appendLine("Device: ${Build.MANUFACTURER} ${Build.MODEL} (${Build.DEVICE})")
        appendLine("Android Version: ${Build.VERSION.RELEASE} (SDK ${Build.VERSION.SDK_INT})")
        appendLine("Supported ABIs: ${Build.SUPPORTED_ABIS.contentToString()}")
        appendLine("Exception: ${throwable.javaClass.name}")
        appendLine("Message: ${throwable.message}")
        appendLine("Cause: ${throwable.cause}")
        appendLine("----------------- FULL STACKTRACE -----------------")
        appendLine(stackTrace)
        appendLine("======================================================")
      }

      Log.e(TAG, report)

      val targetPaths = mutableListOf<File>()
      targetPaths.add(File("/storage/emulated/0/Download/veloxdb_crash.txt"))
      try {
        targetPaths.add(File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "veloxdb_crash.txt"))
      } catch (_: Exception) {}

      context?.let { ctx ->
        try {
          ctx.getExternalFilesDir(null)?.let { dir ->
            targetPaths.add(File(dir, "veloxdb_crash.txt"))
          }
        } catch (_: Exception) {}
        try {
          targetPaths.add(File(ctx.filesDir, "veloxdb_crash.txt"))
        } catch (_: Exception) {}
      }

      for (file in targetPaths) {
        try {
          file.parentFile?.mkdirs()
          file.writeText(report)
          Log.i(TAG, "Crash report successfully written to: ${file.absolutePath}")
        } catch (e: Exception) {
          Log.w(TAG, "Failed writing crash report to ${file.absolutePath}: ${e.message}")
        }
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // 1. Install global uncaught exception handler
    val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()
    Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
      try {
        logCrashToFile(applicationContext, throwable)
      } catch (e: Exception) {
        Log.e(TAG, "Failed inside uncaughtExceptionHandler", e)
      } finally {
        defaultHandler?.uncaughtException(thread, throwable)
      }
    }

    // 2. Pre-load C++ runtime if available to prevent transitive dlopen failures
    try {
      System.loadLibrary("c++_shared")
    } catch (e: Throwable) {
      Log.w(TAG, "libc++_shared preload notice: ${e.message}")
    }

    // 3. Catch startup exceptions and show dialog before force-closing
    try {
      enableEdgeToEdge()
      super.onCreate(savedInstanceState)
    } catch (t: Throwable) {
      logCrashToFile(this, t)
      try {
        AlertDialog.Builder(this)
          .setTitle("VeloxDB Startup Error")
          .setMessage("${t.javaClass.simpleName}:\n${t.message}\n\nCrash log saved to:\n/storage/emulated/0/Download/veloxdb_crash.txt")
          .setPositiveButton("Tutup") { _, _ -> finish() }
          .setCancelable(false)
          .show()
      } catch (_: Exception) {
        throw t
      }
    }
  }
}
