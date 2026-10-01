import fs from "node:fs/promises";

let readerPromise;

async function reader(dbPath) {
  if (!dbPath) return null;
  readerPromise ||= import("maxmind").then(({ open }) => open(dbPath)).catch((error) => {
    console.error("GeoIP database unavailable:", error.message);
    return null;
  });
  return readerPromise;
}

export async function lookupGeo(ipAddress, { dbPath, provider = "local-mmdb" } = {}) {
  if (!ipAddress || !dbPath) return null;
  try {
    await fs.access(dbPath);
    const database = await reader(dbPath);
    const result = database?.get(ipAddress);
    if (!result) return null;
    const country = result.country || {};
    const subdivision = result.subdivisions?.[0] || {};
    const city = result.city || {};
    const location = result.location || {};
    const traits = result.traits || {};
    return {
      countryCode: country.iso_code || null,
      countryName: country.names?.en || null,
      region: subdivision.names?.en || null,
      city: city.names?.en || null,
      latitude: Number.isFinite(location.latitude) ? location.latitude : null,
      longitude: Number.isFinite(location.longitude) ? location.longitude : null,
      timezone: location.time_zone || null,
      asn: traits.autonomous_system_number ? String(traits.autonomous_system_number) : null,
      organization: traits.autonomous_system_organization || null,
      isp: traits.isp || null,
      geoSource: provider,
    };
  } catch {
    return null;
  }
}
