# Profile location catalog

`apis/src/profile_catalog.json` is bundled locally: profile searches do not send queries to a third-party service.

Countries use ISO 3166-1 names and codes from the system iso-codes dataset.
Cities use GeoNames cities15000, downloaded on 2026-10-30, including cities with population >= 100,000 and national capitals.
City labels include the country. Flags are independently selectable and do not imply residence or nationality.

GeoNames data: https://download.geonames.org/export/dump/ (cities15000.zip).
Credit: GeoNames, https://www.geonames.org/, licensed under CC BY 4.0: https://creativecommons.org/licenses/by/4.0/.

No map, precise coordinates, or map visibility settings are stored. Birth year is private to the owner; public responses include only the age reached in the current year. Profile data is erased when an account is deleted.

Flag SVGs: flag-icons 7.6.0, https://github.com/lipis/flag-icons, MIT license (bundled in `apis/assets/profile-flags/LICENSE.txt`).
