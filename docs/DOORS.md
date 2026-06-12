# Doors — the self-describing docking surface

The design law: **every function gets its own addressed door; a chat
window is one door among many, never the door.** The docking surface of
the whole stack is meant to spread — vertically, organ by organ — until a
surface skin (CLI, browser, GUI) is nothing but a projection of the
catalog underneath.

That requires the operator's standing question — *what, exactly, could I
operate right now?* — to have a machine answer that cannot rot. A
hand-written inventory drifts the day it is written. So the system speaks
for itself.

## The vocabulary

`kosmo_core::doors`: a [`Door`] is one operator-facing entry point — its
surface (CLI flag of a binary, HTTP route of a server), its name and
aliases, one summary sentence, its inputs (companion flags / request
fields, with value shapes and required-ness), its **write power as data**
(`read-only` / `writes-workspace` / `appends-store` / `governance-act`)
and its needs (provider, cargo, network, workspace, store, file). Doors
are content-addressed; a [`DoorCatalog`] is the deterministic, deduped,
content-addressed inventory of one surface — and catalogs from several
surfaces merge.

## Surfaces describe themselves

- `kosmo-run --doors` (add `--json` for the machine shape): the binary's
  thirteen doors, spoken from inside the binary.
- `GET /api/doors` on kosmo-server: the server's eight routes, spoken by
  the server.

A surface only ever catalogs **its own** doors. No binary claims truth
about another binary's flags — that would be the hand-written inventory
in disguise.

## The pin: the description cannot drift

Each self-describing surface carries a test that scans its **own parser
or router source** and asserts set-equality with its catalog:

- kosmo-run: every `"--flag"` match arm in `parse_args` must be spoken by
  the catalog (as a door, alias or input), and everything the catalog
  speaks must be parsed. A new flag without a description fails the
  build; a described flag without a mechanism fails the build.
- kosmo-server: every `.route(...)` registration must appear in the
  catalog with the right method, and vice versa.

The catalog is deterministic (same binary ⇒ byte-identical answer,
pinned e2e), needs no workspace, no key, no network.

## Where this goes

The remaining surfaces (kosmo-substrate, kosmo-promote, kosmo-tui, the
pse and mef CLIs) join door by door in later Spreizungen; their catalogs
merge into one ecosystem inventory. A GUI built after that point is
generated *from* the catalog — the skin collapses out of the docking
surface instead of being invented in front of it.
