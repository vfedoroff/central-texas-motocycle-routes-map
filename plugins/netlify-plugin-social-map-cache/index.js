const mapCache = '.cache/social-maps';

export async function onPreBuild({ constants, utils: { cache } }) {
  if (!constants.IS_LOCAL) await cache.restore(mapCache);
}

export async function onEnd({ constants, utils: { cache } }) {
  // Retain completed exports even when a later build step fails.
  if (!constants.IS_LOCAL) await cache.save(mapCache);
}
