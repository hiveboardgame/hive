# Hive maintenance scripts

`script` contains explicit operator utilities. It never runs as part of the
application server.

## Legacy tournament cutover

The cutover tooling separates legacy interpretation, a canonical accepted
bundle, and final-schema import:

```sh
cargo run -p script -- legacy-tournaments audit

cargo run -p script -- legacy-tournaments export \
  --output legacy-tournament-exports/cutover.jsonl \
  --confirm-database-name hive-local \
  --confirm-writes-stopped

cargo run -p script -- legacy-tournaments import \
  --input legacy-tournament-exports/cutover.jsonl \
  --confirm-source-database-name hive-local \
  --confirm-database-name hive-local \
  --confirm-bundle-sha256 '<manifest jsonl_sha256>' \
  --confirm-writes-stopped
```

`DATABASE_URL` must point at the database for the current command.
`--database-url` is available for an explicit connection string, but
credentials must not be placed in shell history or committed files.

The preliminary `audit` is safe to run while the old application is live. It
uses the same classification and translation as export, prints a structured
report, and exits unsuccessfully if it finds an unsupported or malformed
shape. Its result is only a rehearsal because live data can change afterward.

The authoritative export and import are one maintenance-window operation:

1. Stop the application and freeze writes.
2. Run export with the exact value returned by `current_database()`.
3. Review the emitted audit report and the JSON manifest accepted, skipped,
   rejected, and per-record counts plus its checksum. A unique agreement on an
   unstarted game becomes the Slot's scheduled time. One to three future
   proposals from one player become a current offer; expired, conflicting, or
   otherwise ambiguous schedule rows are reported as skipped, not imported.
4. Apply the final migration without reopening application writes.
5. Run import with the manifest's source name, the connected target name, and
   its exact checksum.
6. Verify the summary and representative Tournament pages before reopening
   writes.

Export reads one repeatable-read snapshot and rejects a missing or incompatible
legacy column that is required to interpret Tournament facts. Unrelated
tables, columns, indexes, and migration entries do not affect it. It refuses
an existing data or manifest path, writes private (`0600`) same-directory
temporary files, publishes deterministic JSONL first, and publishes
`<output>.manifest.json` last as the acceptance marker. A bundle without its
manifest is not accepted.

The accepted bundle contains only canonical Tournament records: final Config
and lifecycle, Series relationships, organizers, memberships, invitations,
native Swiss/elimination facts, deterministic Slots, retained Game
ownership, accepted scheduled times, unambiguous current offers, and final
outcomes. For legacy Double-Swiss rounds, each pairing's first-finished game
supplies both stored pre-result ratings; export fails if that pair is missing
or invalid. It does not contain a source schema fingerprint, migration ledger,
chat/history data, or a second legacy representation. Treat it as sensitive
export material. The recommended
`legacy-tournament-exports/` directory and conventional cutover filenames are
gitignored; still verify `git status` before committing.

None of these commands runs Diesel migrations, alters `schema.rs`, or writes
the migration ledger. Import validates the manifest counts,
operator checksum, and both explicit database confirmations. It accepts only
an empty canonical Tournament target, then inserts the canonical records and
relinks retained Games in one transaction and dependency order. Final-schema
foreign keys and CHECK constraints validate relational integrity. Any missing
Game, constraint failure, duplicate, or partial target rolls the entire import
back. Import never reconstructs or re-audits the legacy source and performs no
engine replay.

During legacy interpretation, a Round Robin with every result recorded is
exported as finished even if its old manual lifecycle still says `InProgress`.
Its final standings come from the mapped results, and its finish timestamp is
no earlier than its last recorded result. Round Robins still missing results
remain ongoing, with their retained Games linked to unresolved Slots so normal
game completion can finish the tournament after cutover.

After import, verify the concise imported, retained, skipped, and rejected
counts, Tournament pages, Game links, scheduled times, current offers,
and final standings before reopening writes. Legacy scheduling history is
intentionally not reconstructed.
Re-running import against the populated target fails closed instead of trying
to continue a partial cutover.

## Synthetic tournament QA fixtures

The synthetic fixture command is an explicit local-only operator action. It
requires both the supplied confirmation and PostgreSQL's `current_database()`
to be exactly `hive-local`; it is never called by the application.

```sh
cargo run -p script -- synthetic-tournaments seed \
  --confirm-database-name hive-local

# Reproduce a useful randomized cohort exactly:
cargo run -p script -- synthetic-tournaments seed \
  --confirm-database-name hive-local --seed 123456

cargo run -p script -- synthetic-tournaments arm-due \
  --confirm-database-name hive-local

cargo run -p script -- synthetic-tournaments cleanup \
  --confirm-database-name hive-local
```

Seed creates 162 QA accounts (one organizer, 160 players, and one dedicated
account-deletion test player), all with the
password `hivegame`, and a bounded, deterministic matrix of 24 recognizable
`QA ...` scenarios through tournament services. It prints the reproduction
seed, QA accounts, shared password, tournament IDs/URLs, expected states, and
manual checks. The matrix includes
at least one unstarted, ongoing, and finished fixture for every tournament
format. Large fixtures include maximum-size elimination brackets, a 160-player
Swiss, a 120-player Arena scheduled roughly a year ahead, a maximum-density
finished Round Robin, and varied realtime and correspondence clocks. Live
fixtures have
completed history plus a mixed active frontier rather than stopping at their
opening round. Elimination releases ready matches independently of unrelated
matches, and its started fixtures use custom player placement. Round Robin and
Swiss exercise optional seed tiebreaks; Double Swiss uses the opposite score as
a tiebreak. Finished Swiss fixtures must materialize and resolve
every configured round or the seed transaction fails. The printed seed keeps
participant selection, clocks, pairings, and result details reproducible while
field sizes and lifecycle coverage remain stable. Every generated account starts
with `zz_qa_t_`. If that account namespace is already in use, seed refuses to
run until cleanup succeeds.

The undersubscribed scheduled fixture is initially set to become due 65 seconds
after seed starts. Open its printed URL immediately to observe the future-start
boundary, the due waiting state, and the worker clearing only `starts_at` on its
following sweep. If that transition has already completed, keep the page open
and run `arm-due`; it schedules the same organizer-owned, still-unstarted,
still-undersubscribed fixture 65 seconds ahead for another observation.

Seed, `arm-due`, and cleanup share a transaction advisory lock. Conflict
checking happens under that lock, so overlapping operator commands serialize
instead of reporting a stale cleanup or an incidental uniqueness failure.

The final 33-player bracket review fixture has two organizers and a pending
organizer invitation. Its 15-minute setup belongs to `zz_qa_t_organizer`. Use
`zz_qa_t_delete_test` to check account deletion during setup; that account
participates only in this fixture. After expiry, use Start to open a fresh setup.

Cleanup captures account and tournament IDs registered when seeding in the
operator-only `synthetic_tournament_qa` schema. Those IDs survive account
anonymization and organizer changes. Before deleting anything, cleanup refuses
if a QA account participates in or organizes a tournament outside that cohort.
Shared series are also protected. Resolve those memberships deliberately before
retrying; inviting a QA account never makes a real tournament a cleanup target.

The successful path removes the captured tournament structures, tournament and
casual Games involving the QA accounts, challenges, schedules, chat and email
rows, account preferences/devices/ratings/blocks, and the QA accounts in one
transaction. It verifies that no captured account or reference remains before
committing. Unrelated rows are preserved; cleanup does not truncate permanent
tables, run migrations, or reset the database.
