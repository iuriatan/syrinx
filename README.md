# Syrinx

A media ingestion routine for the _canaria_ audio library access system.

Syrinx traverses a filesystem entrypoint linking audio files (along with it's
metadata) to a library node stored in a [Dgraph](https://dgraph.io) backend.

## Configuration

Syrinx configuration parameters can be defined in a `Syrinx.yaml` and/or by
environment variables, which takes precedence over file configuration.
