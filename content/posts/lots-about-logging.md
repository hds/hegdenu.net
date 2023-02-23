+++
title = "lots about logging"
slug = "lots-about-logging"
author = "hds"
date = "2023-02-23"
draft = true
+++

Logging could be defined as "printf debugging in production".

And we all know how useful printf debugging is.

The difference in what you get from bad logging compared and good logging can be huge.

This post describes some ways that you can get the most out of your logs.

### what we'll talk about

Logs in the context of this post are text output from your application.

We're not talking about tracing, distributed or otherwise.

No structure between logs is expected.

That's not to say that these things aren't nice, they're wonderful!

But sometimes you just don't have those tools.

So let's talk about what to do with basic tools.

For continuity, we'll follow two examples throughout.

#### back-end server

This could be a web server, or something further "back" in the stack.

Most people have worked on a web server.

Or played with one.

Traditionally the logs into access logs (`stdout`) and error logs (`stderr`).

And the famous Apache [Common Log Format](https://httpd.apache.org/docs/2.4/logs.html#common).

```
127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326 
```

We expect server logs to be available immediately, have high volume per host, and high concurrency.

#### mobile application

We'll use mobile as a (these days) common front-end application.

These logs may only be available in crash reports, and certainly not immediately.

I mean, I **hope** you're not sending large volumes of logs from an app running on my phone.

We would also expect low concurrency

A more extreme example would be an application running on an embedded device or in a car.

### what log levels mean

Most logging frameworks have at least the following levels (from most to least severe):

* `ERROR`
* `WARNING` (often `WARN`)
* `INFO`
* `DEBUG`

At the most severe end, you may also have `FATAL`.

At the least severe end, you may also have `TRACE`.

Log levels are often misused, resulting in either too many logs or not enough.

That balance can be tricky.

Here are some pointers

#### not every error is an `ERROR`

There's nothing worse than not being able to find the important `ERROR` because your logs are full of unimportant ones.

Personally, I prefer to err on the side of caution.

(Pun absolutely intended.)

If the error can be handled, it's not an `ERROR`.

If your web server returns a 4xx status code, it's not an `ERROR`.

Actually, if your web server returns a 5xx status code, it's probably not an `ERROR` either.

This is because you can find 5xx errors in your logs anyway, you have a field for that.

You do have a field for that, right? (if not, see [structured logging is your friend](#structured-logging-is-your-friend).)

If your mobile app can't connect to a backend service because there's no network, it's not an `ERROR`.

Your mobile app is used to being without network.

If the user gives you weird input it's not an `ERROR`.

An `ERROR` should appear just before the logs stop because the whole system fell over.

For example, in non-garbage collected languages, not being able to allocate memory may be an `ERROR`.

Especially if the next thing your program does is crash.

If you have a `FATAL` level, this doesn't change.

It just means that you've got a different level for your actual stack trace.

Or somewhere to write:

```
Segmentation fault (core dumped)
```

#### it's not a `WARNING` either

If after reading the last section you're converting all your old `ERROR` logs into `WARN`, stop.

They might not be a `WARNING` either.

Consider whether the information in the log message is actionable.

Will this `WARNING` alert me to something that I can fix now (or in the next mobile app store release)?

If it isn't, probably no need to write a `WARNING`.

The idea here is about usability.

An `ERROR` should tell you why your program broke irreparably.

A `WARNING` should tell you why it's about to break.

If it's not doing this, relegate it further down the severity chain.

#### just don't log it

As you keep pushing items further down the severity chain, you'll get to the bottom.

The catch-all level: `TRACE` (or `DEBUG`)

Now what?

You can leave it there.

In production, and probably even in development, `TRACE` is disabled.

But before you do leave it.

Ask yourself: Will the information here ever actually help me fix something?

Or understand something?

If the answer is no, you know what to do.

### who or what will read your logs

### structured logging is your friend

Before we get into structured logging, what is unstructured logging?

Let's go back to the Common (access) Log Format from our [back-end server](#back-end-server).

```
127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326 
```

I put access in brackets because it's not defined anywhere.

This log contains a bunch of information.

Or it's garbage.

It depends on whether you know what data is where.

If your logs aren't structured, then your log collector needs to know the structure.

That makes changing the structure hard.

It makes removing fields break many things.

Let's modify our log line a little bit to give us context.

```
remote_ip_address=127.0.0.1 identity=- http_user=frank request_date="10/Oct/2000:13:55:36 -0700" http_method=GET http_path=/apache_pb.gif http_protocol=HTTP/1.0 http_status_code=200 response_size_bytes=2326 
```

This turns out to be a bit long, so let's split it onto multiple lines.

But just for readability on a web-site.

Or not. (remember to think about [who or what will read your logs](#who-or-what-will-read-your-logs))

```
remote_ip_address=127.0.0.1
    identity=-
    http_user=frank
    request_date="10/Oct/2000:13:55:36 -0700"
    http_method=GET
    http_path=/apache_pb.gif
    http_protocol=HTTP/1.0
    http_status_code=200
    response_body_bytes=2326 
```

Now our logs are structured!

This means that it doesn't matter if I remove one of the fields.

The rest will still work.

I can reorder them too.

If you're worried about the increased size of the logs, there are options.

#### schema version your unstructured logs


### aggregate logs for readability and usefulness