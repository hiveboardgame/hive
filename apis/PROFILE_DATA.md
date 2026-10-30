# Optional profile data

Profiles store only a voluntarily selected country flag and up to 300 characters of About text. The flag is self-selected and does not establish residence, citizenship, or origin. Both fields are public when filled in and can be cleared. Profile records are deleted on account deletion.

The country catalog in `apis/src/profile_catalog.json` is bundled locally and uses ISO 3166-1 names and codes from the system iso-codes dataset. There are no dedicated profile fields for city, birth year, age, gender, coordinates, or map visibility. Users can still voluntarily include personal information in their public About text.

The simplification migration drops the former location, birth_year, and gender columns, deleting their stored values. Rolling it back restores empty columns only.

About text is HTML-escaped. Web URLs reuse `validated_link_url` and `chat_link_href`, including the existing external-site confirmation screen. This is destination validation and a confirmation flow, not a malware scanner. Images and user-supplied HTML are not rendered.

Flag SVGs: flag-icons 7.6.0, https://github.com/lipis/flag-icons, MIT license (bundled in `apis/assets/profile-flags/LICENSE.txt`).
