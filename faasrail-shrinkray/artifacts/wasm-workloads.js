[
  {
    "mean": 1,
    "stdev": 0.289100,
    "bench": "float",
    "payload": "{\"n\": 10000}"
  },
  {
    "mean": 0,
    "stdev": 0.00,
    "bench": "json",
    "payload": "{\"json_string\": \"{\\\"a\\\":1,\\\"b\\\":[1,2,3]}\"}"
  },
  {
    "mean": 0,
    "stdev": 0.015588,
    "bench": "chameleon",
    "payload": "{\"num_of_cols\": 10, \"num_of_rows\": 100}"
  },
  {
    "mean": 0,
    "stdev": 0.107410,
    "bench": "aes",
    "payload": "{\"message_length\": 256, \"num_iterations\": 10}"
  },
  {
    "mean": 59,
    "stdev": 2.312282,
    "bench": "gzip",
    "payload": "{\"file_size\": 1}"
  },
  {
    "mean": 2,
    "stdev": 0.335586,
    "bench": "disk-seq",
    "payload": "{\"byte_size\": 4096, \"file_size\": 1}"
  },
  {
    "mean": 2,
    "stdev": 0.022891,
    "bench": "disk-rand",
    "payload": "{\"byte_size\": 4096, \"file_size\": 1}"
  }
]
