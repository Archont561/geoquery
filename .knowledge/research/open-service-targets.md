---
type: Research Note
title: "Open Geospatial Service Targets"
description: "Candidate public WMS/WFS, OGC API, STAC, ArcGIS REST, USGS and Copernicus endpoints for Geoquery design spikes."
tags: [research, services, WMS, WFS, OGC-API, STAC, ArcGIS, USGS, Copernicus, Poland]
status: draft
generated: { by: agent/arena, at: 2026-10-05T00:00:00Z }
created: 2026-10-05T00:00:00Z
updated: 2026-10-05T00:00:00Z
id: research/open-service-targets
category: research
refs: [project/product-design, adapters/adapter-architecture, adapters/stac, adapters/ogc, infrastructure/storage, query/planner]
---

# Open Geospatial Service Targets

## Purpose

This note collects public services worth using as **design spikes** for Geoquery. The goal is
not to maintain a permanent service catalog. The goal is to pick representative live systems
that force the query model, adapter contracts, service snapshots and result normalization to
face real protocol differences.

The list was researched from public provider documentation and direct landing-page fetches on
2026-10-05. Treat each endpoint as a target to smoke-test before relying on it in CI: public
services change, rate-limit, move, and sometimes expose metadata openly while requiring an
account for asset download.

---

## Highest-value spike set

Start with this set before expanding. It covers legacy XML/GML, modern JSON/GeoJSON, product
catalog APIs, and EO catalog APIs.

| # | Service | Kind | Why it matters |
|---|---------|------|----------------|
| 1 | Polish PRG WFS | WFS 2.0 / GML | Real Polish national service, EPSG:2180 example, administrative boundaries. |
| 2 | Polish PRNG WFS | WFS / GML | Real gazetteer/geographic names, non-English attributes and names. |
| 3 | Polish soil-agricultural map | WMS | WMS-only thematic layer, useful for map-service and GetCapabilities parsing. |
| 4 | PDOK BRT TOP10NL | OGC API Features/Tiles/Styles | Modern OGC API, JSON, no auth, CRS and tile/style links. |
| 5 | pygeoapi demo | OGC API multi-standard | One landing page exposes collections, STAC, processes and tiles. |
| 6 | Element 84 Earth Search | STAC API | Clean unauthenticated STAC item search with Sentinel/Landsat/DEM/NAIP collections. |
| 7 | Microsoft Planetary Computer | STAC API | Rich catalog, bbox/time search, asset signing caveat. |
| 8 | USGS LandsatLook | STAC API | STAC + CQL2/queryables around Landsat data. |
| 9 | USGS TNM Access | REST product API | Non-OGC product catalog with bbox, datasets, formats and direct download URLs. |
| 10 | Copernicus Data Space STAC/OData | STAC + OData | Same domain exposed by two API styles; STAC for discovery, OData for product search/download workflow. |

---

## Legacy OGC Web Services: Poland Geoportal / GUGiK

These endpoints are useful because they represent real-world national infrastructure rather
than clean demos. They will test WMS/WFS capabilities parsing, axis order, CRS handling,
GML parsing, Polish field names, service names that differ only by case, and layer/type-name
discovery.

| Service | Protocol | Endpoint | Notes | Source |
|---------|----------|----------|-------|--------|
| PRG Administrative Boundaries | WMS | `https://mapy.geoportal.gov.pl/wss/service/PZGIK/PRG/WMS/AdministrativeBoundaries` | Official National Register of Boundaries view service. | [Geoportal PRG](https://www.geoportal.gov.pl/en/data/national-register-of-boundaries/) |
| PRG Administrative Boundaries | WFS | `https://mapy.geoportal.gov.pl/wss/service/PZGIK/PRG/WFS/AdministrativeBoundaries` | Official WFS download service; Geoportal example uses GML and EPSG:2180. | [Geoportal PRG](https://www.geoportal.gov.pl/en/data/national-register-of-boundaries/) |
| PRG data-package access | WMS + GetFeatureInfo | `https://integracja.gugik.gov.pl/cgi-bin/PanstwowyRejestrGranic` | Uses GetFeatureInfo to expose downloadable PRG packages at clicked locations. | [Geoportal PRG](https://www.geoportal.gov.pl/en/data/national-register-of-boundaries/) |
| National Address Numbering integration | WMS | `https://integracja.gugik.gov.pl/cgi-bin/KrajowaIntegracjaNumeracjiAdresowej` | Address-related PRG browsing service listed with PRG. | [Geoportal PRG](https://www.geoportal.gov.pl/en/data/national-register-of-boundaries/) |
| PRNG Geographical Names | WMS | `https://mapy.geoportal.gov.pl/wss/service/PZGiK/PRNG/WMS/GeographicalNames` | Geographic names; official/standardized/other name classes. | [GUGiK PRNG news](https://www.gov.pl/web/gugik/nowa-wersja-uslug-wms-i-wfs-dla-rejestru-prng) |
| PRNG Geographical Names | WFS | `https://mapy.geoportal.gov.pl/wss/service/PZGiK/PRNG/WFS/GeographicalNames` | Download/query equivalent of the PRNG WMS. | [GUGiK PRNG news](https://www.gov.pl/web/gugik/nowa-wersja-uslug-wms-i-wfs-dla-rejestru-prng) |
| Soil-agricultural map | WMS | `https://mapy.geoportal.gov.pl/wss/service/pub/guest/MapaGlebowoRolnicza/MapServer/WMSServer` | Thematic WMS published in Geoportal; originally partial national coverage during rollout. | [Geoportal soil WMS](https://www.geoportal.gov.pl/aktualnosci/usluga-wms-prezentujaca-mape-glebowo-rolnicza-dostepna-w-serwisie-www-geoportal-gov-pl/) |

### Legacy adapter implications

- WMS is a **rendering or context source**, not a feature-result source unless GetFeatureInfo
  is intentionally modeled.
- WFS returns features but likely needs GML parsing, type-name discovery, namespace handling,
  paging/COUNT handling and CRS transformation.
- Polish services are good fixtures for axis-order correctness. PRG examples use EPSG:2180;
  WMS 1.3.0 and WFS CRS names must be recorded exactly in the service snapshot.
- A single provider can expose WMS, WFS and download-package behavior for the same resource;
  `ResourceDescriptor.services[]` needs to carry all access routes rather than collapse them.

---

## Modern OGC API / RESTful OGC services

Use these to test the `ogc-api` adapter separately from old WMS/WFS. They return JSON/GeoJSON,
have landing pages, collections, conformance and often OpenAPI descriptions.

| Service | Endpoint | Useful facets | Source |
|---------|----------|---------------|--------|
| PDOK BRT TOP10NL | `https://api.pdok.nl/kadaster/brt-top10nl/ogc/v1` | OGC API Features, Tiles, Styles, collections, OpenAPI, no auth/cost listed. | [PDOK landing page](https://api.pdok.nl/kadaster/brt-top10nl/ogc/v1), [PDOK article](https://www.pdok.nl/ogc-apis/-/article/basisregistratie-topografie-brt-topnl) |
| pygeoapi demo | `https://demo.pygeoapi.io/master` | Collections, STAC, processes, jobs, tile matrix sets, OpenAPI and conformance. | [pygeoapi demo](https://demo.pygeoapi.io/master) |
| ldproxy Daraa demo | `https://demo.ldproxy.net/daraa` | OGC API Features reference/demo dataset; good for feature paging and item-by-id. | [OGC API workshop](https://ogcapi-workshop.ogc.org/api-deep-dive/features/) |
| ldproxy Zoomstack demo | `https://demo.ldproxy.net/zoomstack` | Multiple collections used by QGIS/GDAL examples. | [OGC API workshop](https://ogcapi-workshop.ogc.org/api-deep-dive/features/) |
| MSC GeoMet OGC API | `https://api.weather.gc.ca/` | OGC API Features and environmental data; bbox, datetime/property queries and CQL2 examples. | [MSC GeoMet docs](https://eccc-msc.github.io/open-data/msc-geomet/ogc_api_en/) |
| Geonovum OGC API Testbed | `https://apitestbed.geonovum.nl/` | Multiple implementations behind one testbed: pygeoapi, ldproxy, GeoServer, QGIS, pycsw. | [Geonovum testbed](https://apitestbed.geonovum.nl/) |

### OGC API adapter implications

- Start with landing page → conformance → collections → collection/items.
- Record `service-desc`/OpenAPI links and conformance classes in snapshots.
- Treat OGC API Features and OGC API Tiles/Styles as related but distinct capabilities.
- GeoMet is especially useful for testing property filters and temporal filters outside EO/STAC.

---

## STAC APIs

These are the best first targets for live EO search because they already align with
Geoquery's spatial/temporal/filter/result model. They also expose asset links and STAC
extensions that stress the `GeoResult.assets`, `properties`, and `raw` decisions.

| Service | Endpoint | Useful facets | Caveats | Source |
|---------|----------|---------------|---------|--------|
| Element 84 Earth Search | `https://earth-search.aws.element84.com/v1` | Public datasets on AWS; landing page advertises `/search`, collections, queryables, fields/sort/query conformance and child collections for Sentinel, Landsat, DEM, NAIP. | Asset access is cloud-object access; verify CORS/rate behavior separately. | [Earth Search landing page](https://earth-search.aws.element84.com/v1) |
| Microsoft Planetary Computer | `https://planetarycomputer.microsoft.com/api/stac/v1` | Rich hosted catalog; documented bbox/datetime search through `pystac-client`. | Some asset URLs require signing/SAS tokens even when metadata search is public. | [Planetary Computer STAC quickstart](https://planetarycomputer.microsoft.com/docs/quickstarts/reading-stac/) |
| NASA CMR-STAC | `https://cmr.earthdata.nasa.gov/stac` | Root STAC catalog over NASA providers; child provider catalogs such as `LPCLOUD`. | Searches are provider-scoped; asset downloads may need Earthdata credentials. | [CMR STAC root](https://cmr.earthdata.nasa.gov/stac), [NASA Openscapes tutorial](https://nasa-openscapes.github.io/2021-Cloud-Hackathon/tutorials/02_Data_Discovery_CMR-STAC_API.html) |
| USGS LandsatLook | `https://landsatlook.usgs.gov/stac-server/` | Landsat STAC API; OpenAPI documents item search, bbox, intersects, datetime, fields, sort, query, CQL2 filter and queryables. | STAC Index notes CORS issues; verify direct client behavior. | [USGS API docs](https://landsatlook.usgs.gov/stac-server/api.html), [STAC Index](https://www.stacindex.org/catalogs/usgs-landsat-collection-2-api) |
| Copernicus Data Space STAC | `https://stac.dataspace.copernicus.eu/v1/` | Current CDSE STAC endpoint; supports filter/query/fields/sort and collection queryables for selected collections. | Legacy `catalogue.dataspace.copernicus.eu/stac` is deprecated; use the new `stac.dataspace...` host. | [CDSE STAC docs](https://documentation.dataspace.copernicus.eu/APIs/STAC.html) |

### STAC adapter implications

- A STAC adapter can be the first real execution path: bbox, datetime, collection, limit,
  fields, sort, query/CQL2 filters, assets and links map naturally to Geoquery.
- Queryables matter. The adapter should discover per-collection queryables before assuming a
  property like `eo:cloud_cover` can be pushed down.
- Public metadata does not guarantee public asset bytes. `AuthDescriptor` and provenance need
  to distinguish metadata search from asset download authorization.

---

## USGS-specific APIs and services

USGS exposes both standards-based services and domain/product APIs. They are valuable because
not every useful source is pure OGC/STAC.

| Service | Kind | Endpoint | Useful facets | Source |
|---------|------|----------|---------------|--------|
| USGS TNM Access datasets | REST product API | `https://tnmaccess.nationalmap.gov/api/v1/datasets` | Dataset catalog includes product titles, formats, refresh cycles, map-server links. | [TNM datasets endpoint](https://tnmaccess.nationalmap.gov/api/v1/datasets) |
| USGS TNM Access products | REST product API | `https://tnmaccess.nationalmap.gov/api/v1/products` | Bbox/dataset/format/date-style product search; returns download URLs, bounding boxes and metadata URLs. | [TNM products endpoint](https://tnmaccess.nationalmap.gov/api/v1/products?offset=0&max=1&outputFormat=JSON) |
| USGS The National Map ArcGIS REST index | ArcGIS REST | `https://index.nationalmap.gov/arcgis/rest/` | Service directory lists index MapServers for 3DEP, NHDPlus HR, NAIP imagery index, US Topo availability. | [USGS ArcGIS REST directory](https://index.nationalmap.gov/arcgis/rest/) |
| USGS National Map service endpoints | REST/WMS/WMTS/WFS/WCS | Provider service index | USGS FAQ states The National Map Services include REST, WMS, WMTS, WFS, WCS and other service links. | [USGS FAQ](https://www.usgs.gov/faqs/where-can-i-find-a-list-urls-national-map-services) |
| USGS LandsatLook STAC | STAC API | `https://landsatlook.usgs.gov/stac-server/` | Same endpoint as above; best USGS EO search candidate. | [USGS LandsatLook STAC docs](https://landsatlook.usgs.gov/stac-server/api.html) |
| USGS M2M | JSON API | `https://m2m.cr.usgs.gov/api/api/json/stable/` | EarthExplorer-style API base; token-authenticated workflow documented through `login-token`. | [M2M token docs](https://d9-wret.s3.us-west-2.amazonaws.com/assets/palladium/production/s3fs-public/media/files/M2M%20Application%20Token%20Documentation_072024.pdf) |

### USGS adapter implications

- TNM is not STAC but returns a product-search result close to `GeoResult` with `downloadURL`,
  `boundingBox`, `format`, `publicationDate`, `metaUrl` and `previewGraphicURL`.
- TNM `datasets` parameters appear to use provider titles; a descriptor should cache the exact
  dataset titles from `/datasets` rather than ask users to guess them.
- ArcGIS REST directories are service registries in practice; an ArcGIS adapter needs to crawl
  folders/services/layers and normalize MapServer/FeatureServer capabilities.
- M2M likely requires authentication for many operations; treat it as lower priority than TNM
  and LandsatLook for an open metadata spike.

---

## Copernicus APIs

Copernicus Data Space has multiple supported APIs. For Geoquery, STAC and OData are the
important discovery/search candidates. OpenSearch should be treated as legacy only.

| API | Endpoint | Useful facets | Caveats | Source |
|-----|----------|---------------|---------|--------|
| CDSE STAC | `https://stac.dataspace.copernicus.eu/v1/` | STAC 1.1.0 catalogue; `/collections`, `/search`, filter/query/fields/sort, per-collection queryables. | Collection coverage is still described as expanding. | [CDSE STAC docs](https://documentation.dataspace.copernicus.eu/APIs/STAC.html) |
| CDSE OData | `https://catalogue.dataspace.copernicus.eu/odata/v1/Products` | REST/OData product search with `$filter`, `$orderby`, `$top`, `$skip`, `$count`, `$expand`; examples include collection, dates, attributes and geometry. | Product download requires auth token workflow; metadata search can still inform registry. | [CDSE OData docs](https://documentation.dataspace.copernicus.eu/APIs/OData.html) |
| CDSE OpenSearch | `https://catalogue.dataspace.copernicus.eu/resto/api` | Historical/legacy catalogue interface. | Decommissioned after 2026-02-02 according to CDSE notice; do not target for new adapter work. | [CDSE upcoming changes](https://documentation.dataspace.copernicus.eu/APIs/Others/UpcomingChanges.html), [decommission notice](https://dataspace.copernicus.eu/news/2025-10-16-opensearch-catalogue-api-decommissioning-notice) |
| Sentinel Hub Catalog | Sentinel Hub APIs under CDSE | STAC-like Catalog API through Sentinel Hub client ecosystem. | Usually tied to Sentinel Hub account/config. | [CDSE migration guide](https://documentation.dataspace.copernicus.eu/notebook-samples/sentinelhub/migration_from_scihub_guide.html) |

### Copernicus adapter implications

- Prefer STAC for Geoquery's generic EO path.
- OData is worth a separate adapter only if Geoquery wants richer CDSE product/download flows.
- Do not design around OpenSearch except perhaps as a read-only migration note.
- Credential handling must separate catalogue search from product download.

---

## ArcGIS REST / Esri services

ArcGIS REST should be its own adapter family. A MapServer can draw/query layers; a
FeatureServer can return features. Some services also expose WMS/WMTS, but the ArcGIS REST
JSON schema is often the richer contract.

| Service | Endpoint | Useful facets | Source |
|---------|----------|---------------|--------|
| USGS National Map index directory | `https://index.nationalmap.gov/arcgis/rest/` | Folder/service discovery, MapServers for availability/index layers. | [USGS ArcGIS REST directory](https://index.nationalmap.gov/arcgis/rest/) |
| FEMA NFHL public service | `https://hazards.fema.gov/gis/nfhl/rest/services/public/NFHL/MapServer` | Flood-hazard MapServer with layer query potential; common public GIS source. | [Esri community example](https://community.esri.com/t5/arcgis-javascript-maps-sdk-questions/using-fema-rest-end-point/td-p/377621), [FEMA GIS links](https://gis.fema.gov/) |
| ArcGIS REST sample FeatureServer | `https://sampleserver10.arcgisonline.com/arcgis/rest/services/USA/FeatureServer` | Canonical FeatureServer resource shape for layer/table discovery. | [ArcGIS Feature Service docs](https://resources.arcgis.com/en/help/sds/rest/featureService.html) |
| ArcGIS REST sample MapServer | `https://sampleserver1.arcgisonline.com/ArcGIS/rest/services/Specialty/ESRI_StateCityHighway_USA/MapServer` | Canonical MapServer resource shape and operations. | [ArcGIS Map Service docs](https://developers.arcgis.com/rest/services-reference/enterprise/map-service/) |

### ArcGIS adapter implications

- Treat `FeatureServer` as feature-query capable; treat `MapServer` as possibly query-capable
  per layer but often rendering-first.
- Service snapshots should store layer ids, names, geometry types, extents, spatial reference,
  `maxRecordCount`, `capabilities`, and supported query formats.
- ArcGIS where clauses are an attribute-filter compilation target, not a source language.

---

## What this means for Geoquery design

1. **One adapter is not enough.** STAC is the best first execution adapter, but Poland's
   Geoportal and USGS services show that WMS/WFS, ArcGIS REST and product APIs remain real.
2. **Snapshots are not optional for trust.** Live services expose capability and schema facts
   that users cannot remember: CRS, type names, queryables, auth, limits, layer ids and
   paging. Describe-once snapshots make those facts inspectable.
3. **`ServiceDescriptor` needs protocol-specific metadata.** Keep the core fields stable, but
   preserve raw layer/type/collection/queryable facts under `metadata` or typed extension
   sections so adapters do not lose the data they need for execution.
4. **Result normalization must be tested with real payloads.** STAC Items, TNM product records,
   ArcGIS features and WFS GML features will not naturally carry identical fields. `raw` and
   `properties` remain important escape hatches.
5. **Auth is per-operation.** A service can allow anonymous search but require credentials for
   asset download. `AuthDescriptor` should distinguish search, asset signing and download.
6. **WMS should not masquerade as search.** WMS can be represented as render/context capability;
   WFS, OGC API Features, STAC, ArcGIS FeatureServer and product APIs are the better result
   sources.

---

## Spike checklist

For each endpoint selected for a spike, record these facts in the spike result:

- landing/capabilities URL and response media type;
- advertised collections/layers/type names;
- supported spatial filters and expected CRS/axis order;
- temporal filter support and date field names;
- attribute filter/queryables support;
- paging/limit semantics and max page size;
- whether metadata search is anonymous;
- whether asset/download access requires credentials;
- one tiny query over a known bbox and the raw response shape;
- how the response maps to `GeoResult`, including what must stay in `raw`.

Suggested Warsaw bbox for quick Polish/EO tests in CRS84:

```json
[20.85, 52.10, 21.25, 52.35]
```

Suggested first STAC query shape:

```json
{
  "bbox": [20.85, 52.10, 21.25, 52.35],
  "datetime": "2024-05-01T00:00:00Z/2024-09-30T23:59:59Z",
  "collections": ["sentinel-2-l2a"],
  "limit": 10
}
```

---

## Source index

- [Geoportal PRG](https://www.geoportal.gov.pl/en/data/national-register-of-boundaries/)
- [GUGiK PRNG WMS/WFS announcement](https://www.gov.pl/web/gugik/nowa-wersja-uslug-wms-i-wfs-dla-rejestru-prng)
- [Geoportal soil-agricultural WMS announcement](https://www.geoportal.gov.pl/aktualnosci/usluga-wms-prezentujaca-mape-glebowo-rolnicza-dostepna-w-serwisie-www-geoportal-gov-pl/)
- [PDOK TOP10NL OGC API landing page](https://api.pdok.nl/kadaster/brt-top10nl/ogc/v1)
- [PDOK TOPNL OGC APIs](https://www.pdok.nl/ogc-apis/-/article/basisregistratie-topografie-brt-topnl)
- [OGC API workshop examples](https://ogcapi-workshop.ogc.org/api-deep-dive/features/)
- [MSC GeoMet OGC API docs](https://eccc-msc.github.io/open-data/msc-geomet/ogc_api_en/)
- [pygeoapi demo](https://demo.pygeoapi.io/master)
- [Geonovum OGC API Testbed](https://apitestbed.geonovum.nl/)
- [Element 84 Earth Search landing page](https://earth-search.aws.element84.com/v1)
- [Microsoft Planetary Computer STAC quickstart](https://planetarycomputer.microsoft.com/docs/quickstarts/reading-stac/)
- [NASA CMR-STAC root](https://cmr.earthdata.nasa.gov/stac)
- [NASA Openscapes CMR-STAC tutorial](https://nasa-openscapes.github.io/2021-Cloud-Hackathon/tutorials/02_Data_Discovery_CMR-STAC_API.html)
- [USGS LandsatLook STAC docs](https://landsatlook.usgs.gov/stac-server/api.html)
- [USGS LandsatLook STAC Index page](https://www.stacindex.org/catalogs/usgs-landsat-collection-2-api)
- [USGS TNM datasets endpoint](https://tnmaccess.nationalmap.gov/api/v1/datasets)
- [USGS TNM products endpoint](https://tnmaccess.nationalmap.gov/api/v1/products?offset=0&max=1&outputFormat=JSON)
- [USGS ArcGIS REST directory](https://index.nationalmap.gov/arcgis/rest/)
- [USGS National Map services FAQ](https://www.usgs.gov/faqs/where-can-i-find-a-list-urls-national-map-services)
- [CDSE STAC docs](https://documentation.dataspace.copernicus.eu/APIs/STAC.html)
- [CDSE OData docs](https://documentation.dataspace.copernicus.eu/APIs/OData.html)
- [CDSE upcoming changes](https://documentation.dataspace.copernicus.eu/APIs/Others/UpcomingChanges.html)
- [CDSE OpenSearch decommission notice](https://dataspace.copernicus.eu/news/2025-10-16-opensearch-catalogue-api-decommissioning-notice)
- [USGS M2M Application Token Documentation](https://d9-wret.s3.us-west-2.amazonaws.com/assets/palladium/production/s3fs-public/media/files/M2M%20Application%20Token%20Documentation_072024.pdf)
- [ArcGIS Feature Service docs](https://resources.arcgis.com/en/help/sds/rest/featureService.html)
- [ArcGIS Map Service docs](https://developers.arcgis.com/rest/services-reference/enterprise/map-service/)
- [FEMA GIS links](https://gis.fema.gov/)
