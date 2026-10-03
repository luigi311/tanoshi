# Configuration

## Extension concurrency

Installed sources share one extension process per source. Read requests from all
users and background jobs can run concurrently, with a default limit of 8 calls
per source. Each extension's own rate limiter still controls request pacing.

Each source has an independent queue in the VM. User activity, including manual
refreshes, has high priority; downloads and scheduled updates have low priority.
When a slot becomes available, queued high-priority requests run first. Requests
with the same priority run in arrival order. Running calls finish normally.

Requests wait for a slot without an admission timeout. Metadata and image
deadlines start after a slot is acquired and include time spent waiting in the
extension's rate limiter. After admission, preference changes wait in the host
until running reads finish and hold back new reads while the update completes.
That wait uses the metadata deadline; a timeout while waiting for reads to finish
does not retire the worker or count as a source failure.

Configure the limits in `config.yml`:

```yaml
extension:
  max_concurrent_calls_per_source: 8
  metadata_timeout_secs: 30
  image_timeout_secs: 120
```

The global `max_concurrent_calls_per_source` setting currently applies the same
limit to every source independently. The planned design will remove this global
concurrency setting and dynamically determine each source's concurrency from the
rate limits defined by its extension.

An executing call that times out retires its worker. Other running calls get up
to one second to finish before the process is terminated. Reads and preference
replacements interrupted by timeout recovery retry within their original
deadlines. Calls interrupted by an unattributed crash get at most one automatic
crash retry per request, even if successful peers reset the source's health
counter. Calls still queued in the host can retry without consuming that crash
allowance. A replacement starts after the old process exits and replays its
acknowledged preferences. Retries check current source health before replacement
and dispatch; a quarantined source rejects further work.

A request that expires before writing any bytes returns a queue timeout and
leaves the worker running. Expired writer entries are removed before enqueueing
new work, and a full queue reports backpressure rather than a worker crash.
A timeout during a partial frame terminates the process immediately because the
request stream cannot safely continue. A fully written frame keeps the normal
drain period even if flush misses its deadline. Timeouts do not quarantine a
source. An unattributed crash with pending calls counts as one health failure;
idle worker exits do not count. Three consecutive health failures without a
successful call quarantine the source until it is reloaded. Successful calls
reset that health counter, but never a request's crash-retry allowance.
