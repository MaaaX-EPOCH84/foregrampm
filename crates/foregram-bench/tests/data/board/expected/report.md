# Benchmark report

Engines: dot 16.1.0, elk 0.12.0.

Lengths and areas are in hundredths of a pixel, proportion and density in thousandths. Quantiles take the nearest rank, without interpolation: the median is the value of rank ⌈n/2⌉ in increasing order, the 90th percentile that of rank ⌈9n/10⌉, the maximum that of rank n.

## rome (2 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 2 | 2 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 1 | 0 | 1 | 1 | 1 | 0 | 1 | 1 |
| shared_segments | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_length | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 6 | 0 | 6 | 6 |
| edge_length | 20876 | 3600 | 17276 | 17276 | 26668 | 2000 | 24668 | 24668 |
| longest_edge | 8638 | 3600 | 5038 | 5038 | 10334 | 2000 | 8334 | 8334 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| width | 16700 | 4950 | 11750 | 11750 | 25800 | 11900 | 13900 | 13900 |
| height | 21200 | 10600 | 10600 | 10600 | 12500 | 3500 | 9000 | 9000 |
| area | 177020000 | 52470000 | 124550000 | 124550000 | 166750000 | 41650000 | 125100000 | 125100000 |
| proportion | 1575 | 467 | 1108 | 1108 | 4944 | 1544 | 3400 | 3400 |
| density | 1216 | 556 | 660 | 660 | 1386 | 554 | 832 | 832 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 2 graphs laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 0 | 2 | 0 |
| bends | 1 | 1 | 0 |
| edge_length | 1 | 0 | 1 |
| area | 1 | 0 | 1 |

## north (1 graph)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 1 | 1 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_segments | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_length | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 4 | 4 | 4 | 4 |
| edge_length | 18462 | 18462 | 18462 | 18462 | 16116 | 16116 | 16116 | 16116 |
| longest_edge | 10772 | 10772 | 10772 | 10772 | 12116 | 12116 | 12116 | 12116 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| width | 7550 | 7550 | 7550 | 7550 | 18850 | 18850 | 18850 | 18850 |
| height | 17700 | 17700 | 17700 | 17700 | 4500 | 4500 | 4500 | 4500 |
| area | 133635000 | 133635000 | 133635000 | 133635000 | 84825000 | 84825000 | 84825000 | 84825000 |
| proportion | 427 | 427 | 427 | 427 | 4189 | 4189 | 4189 | 4189 |
| density | 389 | 389 | 389 | 389 | 613 | 613 | 613 | 613 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 1 graph laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 0 | 1 | 0 |
| bends | 1 | 0 | 0 |
| edge_length | 0 | 0 | 1 |
| area | 0 | 0 | 1 |

## elk-models (2 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 0 | 2 |
| refused | 2 | 0 |
| refused with `LYT016` | 1 | 0 |
| refused with `REF003` | 1 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | – | – | – | – | 0 | 0 | 0 | 0 |
| shared_segments | – | – | – | – | 1 | 0 | 1 | 1 |
| shared_length | – | – | – | – | 1000 | 0 | 1000 | 1000 |
| bends | – | – | – | – | 2 | 0 | 2 | 2 |
| edge_length | – | – | – | – | 11000 | 2000 | 9000 | 9000 |
| longest_edge | – | – | – | – | 9000 | 2000 | 7000 | 7000 |
| node_overlaps | – | – | – | – | 0 | 0 | 0 | 0 |
| node_overlap_area | – | – | – | – | 0 | 0 | 0 | 0 |
| edge_node_overlaps | – | – | – | – | 0 | 0 | 0 | 0 |
| label_overlaps | – | – | – | – | 0 | 0 | 0 | 0 |
| width | – | – | – | – | 20200 | 10000 | 10200 | 10200 |
| height | – | – | – | – | 11000 | 3000 | 8000 | 8000 |
| area | – | – | – | – | 111600000 | 30000000 | 81600000 | 81600000 |
| proportion | – | – | – | – | 4608 | 1275 | 3333 | 3333 |
| density | – | – | – | – | 1241 | 441 | 800 | 800 |
| layer_violations | – | – | – | – | 0 | 0 | 0 | 0 |
| order_violations | – | – | – | – | 0 | 0 | 0 | 0 |
| port_order_violations | – | – | – | – | 0 | 0 | 0 | 0 |

No graph was laid out by both engines.

## mcd (1 graph)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 1 | 1 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_segments | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_length | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 2 | 2 | 2 | 2 |
| edge_length | 8426 | 8426 | 8426 | 8426 | 22667 | 22667 | 22667 | 22667 |
| longest_edge | 4408 | 4408 | 4408 | 4408 | 17967 | 17967 | 17967 | 17967 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| width | 28500 | 28500 | 28500 | 28500 | 29600 | 29600 | 29600 | 29600 |
| height | 30200 | 30200 | 30200 | 30200 | 30400 | 30400 | 30400 | 30400 |
| area | 860700000 | 860700000 | 860700000 | 860700000 | 899840000 | 899840000 | 899840000 | 899840000 |
| proportion | 944 | 944 | 944 | 944 | 974 | 974 | 974 | 974 |
| density | 487 | 487 | 487 | 487 | 466 | 466 | 466 | 466 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 1 graph laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 0 | 1 | 0 |
| bends | 1 | 0 | 0 |
| edge_length | 1 | 0 | 0 |
| area | 1 | 0 | 0 |
