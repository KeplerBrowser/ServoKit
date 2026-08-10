package org.servo.servokit.reactnative.example

import android.content.Context
import android.content.res.AssetManager
import android.util.Log
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.SocketException
import java.net.URLDecoder
import kotlin.concurrent.thread

object ExampleFixtureServer {
  private const val TAG = "ExampleFixtureServer"
  private const val FIXTURE_ASSET_ROOT = "servo-fixtures"
  private const val HOST = "127.0.0.1"
  private const val PORT = 8481

  const val baseUrl: String = "http://$HOST:$PORT"

  @Volatile
  private var started = false

  fun start(context: Context) {
    if (started) {
      return
    }

    synchronized(this) {
      if (started) {
        return
      }

      val serverSocket =
        try {
          ServerSocket().apply {
            reuseAddress = true
            bind(InetSocketAddress(InetAddress.getByName(HOST), PORT))
          }
        } catch (error: Exception) {
          Log.e(TAG, "Failed to start fixture server on $baseUrl", error)
          return
        }

      started = true

      val assetManager = context.applicationContext.assets
      thread(name = "ServoKitExampleFixtureServer", isDaemon = true) {
        try {
          while (!serverSocket.isClosed) {
            val socket =
              try {
                serverSocket.accept()
              } catch (_: SocketException) {
                break
              }

            thread(name = "ServoKitExampleFixtureRequest", isDaemon = true) {
              handleConnection(assetManager, socket)
            }
          }
        } catch (error: Exception) {
          Log.e(TAG, "Fixture server stopped unexpectedly", error)
        } finally {
          runCatching { serverSocket.close() }
          started = false
        }
      }
    }
  }

  private fun handleConnection(assetManager: AssetManager, socket: Socket) {
    socket.use { connection ->
      val reader = BufferedReader(InputStreamReader(connection.getInputStream(), Charsets.US_ASCII))
      val requestLine = reader.readLine() ?: return
      while (true) {
        val headerLine = reader.readLine() ?: break
        if (headerLine.isEmpty()) {
          break
        }
      }

      val parts = requestLine.split(' ')
      val method = parts.getOrNull(0)
      val rawPath = parts.getOrNull(1)

      when {
        method != "GET" -> {
          writeResponse(
            connection,
            status = "405 Method Not Allowed",
            body = "Only GET is supported.",
            contentType = "text/plain; charset=utf-8"
          )
        }
        rawPath == null -> {
          writeResponse(
            connection,
            status = "400 Bad Request",
            body = "Request path was missing.",
            contentType = "text/plain; charset=utf-8"
          )
        }
        else -> {
          val assetPath = assetPathFor(rawPath)
          if (assetPath == null) {
            writeResponse(
              connection,
              status = "404 Not Found",
              body = "Fixture was not found.",
              contentType = "text/plain; charset=utf-8"
            )
            return
          }

          val body =
            try {
              assetManager.open(assetPath).use { input -> input.readBytes() }
            } catch (_: Exception) {
              writeResponse(
                connection,
                status = "404 Not Found",
                body = "Fixture was not found.",
                contentType = "text/plain; charset=utf-8"
              )
              return
            }

          writeResponse(
            connection,
            status = "200 OK",
            body = body,
            contentType = contentTypeFor(assetPath)
          )
        }
      }
    }
  }

  private fun assetPathFor(rawPath: String): String? {
    val decodedPath = URLDecoder.decode(rawPath.substringBefore('?'), Charsets.UTF_8.name())
    val normalizedPath =
      decodedPath
        .removePrefix("/")
        .ifEmpty { "smoke/index.html" }

    if (normalizedPath.contains("..")) {
      return null
    }

    return "$FIXTURE_ASSET_ROOT/$normalizedPath"
  }

  private fun contentTypeFor(assetPath: String): String =
    when {
      assetPath.endsWith(".html") -> "text/html; charset=utf-8"
      assetPath.endsWith(".css") -> "text/css; charset=utf-8"
      assetPath.endsWith(".js") -> "application/javascript; charset=utf-8"
      else -> "application/octet-stream"
    }

  private fun writeResponse(
    socket: Socket,
    status: String,
    body: String,
    contentType: String
  ) {
    writeResponse(socket, status, body.toByteArray(Charsets.UTF_8), contentType)
  }

  private fun writeResponse(
    socket: Socket,
    status: String,
    body: ByteArray,
    contentType: String
  ) {
    try {
      socket.getOutputStream().use { output ->
        output.write("HTTP/1.1 $status\r\n".toByteArray(Charsets.US_ASCII))
        output.write("Content-Type: $contentType\r\n".toByteArray(Charsets.US_ASCII))
        output.write("Content-Length: ${body.size}\r\n".toByteArray(Charsets.US_ASCII))
        output.write("Connection: close\r\n".toByteArray(Charsets.US_ASCII))
        output.write("\r\n".toByteArray(Charsets.US_ASCII))
        output.write(body)
        output.flush()
      }
    } catch (_: SocketException) {
      // Servo may close fixture responses early once it has what it needs.
    }
  }
}
