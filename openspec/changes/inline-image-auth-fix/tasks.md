## Tasks

- [ ] Investigate: add debug logging to `_attachmentThumbnails()` to confirm whether `imageAuthToken` is null at render time
- [ ] Move `imageBaseUrl` and `imageAuthToken` from `ref.read()` inside `itemBuilder` to `ref.watch()` at the `build()` level so the list rebuilds when auth becomes available
- [ ] Add null guard: if `imageBaseUrl` is null, render a placeholder instead of attempting image load
- [ ] Replace `errorWidget` in `CachedNetworkImage` with a tappable retry widget that calls `CachedNetworkImage.evictFromCache(url)` and triggers rebuild
- [ ] Verify `CachedNetworkImage` sends `httpHeaders` on the actual HTTP request (not just the cache lookup) — add integration test or manual verification
- [ ] Test: load conversation with images, verify thumbnails load with correct auth
- [ ] Test: hard reload on a conversation with images, verify thumbnails load after auth resolves
- [ ] Test: tap retry on a failed image, verify it reloads successfully
- [ ] Test: verify full-size image dialog also sends auth headers correctly
