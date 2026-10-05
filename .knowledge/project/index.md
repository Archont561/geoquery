# Project Foundation

Core identity, architecture, standards alignment, and data model.

* [Geoquery — Project Overview](overview.md) - Mission, what Geoquery is and is not, and the four core abstractions.
* [Product Design Direction](product-design.md) - User loops, interaction principles, and which inspirations to borrow from.
* [Internal Architecture](architecture.md) - Internal architecture diagram and component responsibilities.
* [Standards Position](standards.md) - OGC API, STAC, CQL2, DCAT, GeoJSON, MCP — the standards reuse policy.
* [Core Data Model](data-model.md) - ResourceDescriptor, ServiceDescriptor, CapabilitySet, and open enums.

# Spike Reports

What a particular implementation pass proved, and what it deliberately left open.

* [Phase 1 spike: a live STAC source through the geoquery CLI](phase-1-stac-spike.md) - the first end-to-end slice: registry, one STAC source, execution policy, partial failure.
* [Conformance-driven pushdown](phase-1-stac-pushdown.md) - CQL2 filtering, sorting, field projection and pagination, each gated on what the service declared.

# Navigation

* [Bundle index](../index.md) - master directory of the Geoquery knowledge base.
* [Context briefing](../CONTEXT.md) - single-file mental model of Geoquery.
