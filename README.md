<!--# faasrail-->
```text
                '||''''|                .|'''|  '||'''|,              '||`
                 ||  .                  ||       ||   ||          ''   ||
                 ||''|   '''|.   '''|.  `|'''|,  ||...|'  '''|.   ||   ||
                 ||     .|''||  .|''||   .   ||  || \\   .|''||   ||   ||
                .||.    `|..||. `|..||.  |...|' .||  \\. `|..||. .||. .||.
```

WIP

Extended FaaSRail support for wasm/OpenWhisk backends:
- ow-loadgen
- wasm-loadgen

Included benchmarking script to obtain system metrics for OW from its exposed Prometheus endpoint and API data (from outputs/results.ndjson).

1. Register workloads
```
# Rust
RUNS=10 ./ow-profile.sh ../faasrail-shrinkray/artifacts/ow-workloads.json
```

2. Run shrinkray (generates .csv)
Follow README in /faasrail/faasrail-shrinkray

3. Follow README in /faasrail/ow-loadgen

Obtain results (and perform further benchmarking) from `results.ndjson` in `/outputs` and `ow-bench-collect.py`.

Convenient cURL commands

```
curl -s -X POST -H 'Content-Type: application/json'   -d '{"url": "https://static.vecteezy.com/system/resources/previews/051/679/529/non_2x/sliced-red-strawberry-fruit-on-transparent-background-free-png.png", "hash": "deadbeef", "filename": "strawberry.png", "max_iter": 100}'   http://127.0.0.1:8080/

curl -s -X POST -H 'Content-Type: application/json'   -d '{"url": "https://static.vecteezy.com/system/resources/previews/051/679/529/non_2x/sliced-red-strawberry-fruit-on-transparent-background-free-png.png", "hash": "deadbeef", "filename": "strawberry.png", "max_iter": 100, "data_dependency_path": "test"}'   http://127.0.0.1:8080/
```