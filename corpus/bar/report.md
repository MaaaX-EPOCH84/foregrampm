# Benchmark report

Engines: dot 16.1.0, elk 0.12.0.

Lengths and areas are in hundredths of a pixel, proportion and density in thousandths. Quantiles take the nearest rank, without interpolation: the median is the value of rank ⌈n/2⌉ in increasing order, the 90th percentile that of rank ⌈9n/10⌉, the maximum that of rank n.

## rome (11534 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 11534 | 11534 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 499643 | 22 | 118 | 432 | 763470 | 32 | 184 | 649 |
| shared_segments | 461 | 0 | 0 | 16 | 0 | 0 | 0 | 0 |
| shared_length | 91754 | 0 | 0 | 15384 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 1133892 | 88 | 188 | 306 |
| edge_length | 13054209643 | 865346 | 2482638 | 5851302 | 21976255593 | 1309656 | 4552713 | 10370421 |
| longest_edge | 1013885529 | 81078 | 156092 | 380207 | 1398463695 | 109433 | 221488 | 464378 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| width | 1169430750 | 94375 | 173700 | 453755 | 1130495000 | 95350 | 147150 | 247400 |
| height | 960188200 | 81600 | 124200 | 259100 | 1039753255 | 80025 | 166033 | 243942 |
| area | 103668976581000 | 7643700000 | 17837761500 | 39729248000 | 116199356677600 | 7489370000 | 22394114800 | 39981245850 |
| proportion | 15029467 | 1150 | 2170 | 14224 | 15683945 | 1159 | 2238 | 14674 |
| density | 1401669 | 113 | 178 | 378 | 1459618 | 110 | 211 | 558 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 11534 graphs laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 9079 | 1504 | 951 |
| bends | 11534 | 0 | 0 |
| edge_length | 10920 | 0 | 614 |
| area | 5718 | 0 | 5816 |

## north (1277 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 1277 | 1277 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 75907 | 3 | 140 | 2117 | 69092 | 2 | 105 | 2571 |
| shared_segments | 1444 | 0 | 0 | 154 | 0 | 0 | 0 | 0 |
| shared_length | 102013 | 0 | 0 | 9972 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 91698 | 44 | 162 | 724 |
| edge_length | 1325784074 | 327904 | 2544519 | 31749692 | 2160040837 | 493231 | 4532838 | 47353530 |
| longest_edge | 89929464 | 40197 | 176749 | 544925 | 117753710 | 57400 | 207755 | 729493 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| width | 114519332 | 60950 | 202155 | 480955 | 107417150 | 61600 | 162150 | 582800 |
| height | 87269700 | 46100 | 138400 | 592800 | 87952001 | 47500 | 152878 | 394683 |
| area | 7400106320400 | 3003482700 | 12618645000 | 82716019400 | 8188800454450 | 2836200000 | 16910212500 | 104163227950 |
| proportion | 2802544 | 1247 | 5274 | 45373 | 3130944 | 1299 | 4939 | 77967 |
| density | 189944 | 146 | 228 | 519 | 203946 | 154 | 263 | 733 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 1277 graphs laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 367 | 578 | 332 |
| bends | 1273 | 4 | 0 |
| edge_length | 1173 | 0 | 104 |
| area | 474 | 0 | 803 |

## elk-models (898 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 559 | 878 |
| refused | 339 | 20 |
| refused with `LYT009` | 3 | 0 |
| refused with `LYT015` | 0 | 19 |
| refused with `LYT016` | 310 | 0 |
| refused with `REF003` | 26 | 1 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 24150 | 4 | 59 | 5003 | 13218 | 0 | 9 | 3162 |
| shared_segments | 3823 | 0 | 1 | 1981 | 27416 | 5 | 52 | 2075 |
| shared_length | 1064845 | 0 | 172 | 327371 | 250660286 | 8850 | 197200 | 56422732 |
| bends | 0 | 0 | 0 | 0 | 31075 | 13 | 68 | 906 |
| edge_length | 322273004 | 146165 | 791612 | 42143887 | 551715996 | 116286 | 873328 | 43506155 |
| longest_edge | 19205312 | 17518 | 77230 | 679590 | 50625553 | 24717 | 127300 | 1245867 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 91 | 0 | 0 | 19 | 0 | 0 | 0 | 0 |
| label_overlaps | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 3 |
| width | 30655260 | 40276 | 125599 | 1281621 | 63929677 | 50400 | 145000 | 1304600 |
| height | 24148727 | 34800 | 91800 | 490500 | 31753544 | 23267 | 73317 | 789800 |
| area | 2842890457983 | 1614265000 | 7611681000 | 241379464500 | 5505599937874 | 1334400000 | 7798677068 | 387651453200 |
| proportion | 109343629 | 1014 | 4279 | 40393000 | 126141212 | 1973 | 5453 | 22012000 |
| density | 82326 | 107 | 324 | 1000 | 154473 | 153 | 368 | 1000 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 6502 | 2 | 29 | 303 | 22 | 0 | 0 | 4 |

On the 548 graphs laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 3 | 215 | 330 |
| bends | 449 | 99 | 0 |
| edge_length | 87 | 35 | 426 |
| area | 120 | 8 | 420 |

## mcd (4 graphs)

| graphs | dot | elk |
|---|---:|---:|
| laid out and valid | 4 | 4 |
| refused | 0 | 0 |

| measure | dot total | dot median | dot 90th | dot max | elk total | elk median | elk 90th | elk max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| crossings | 23 | 1 | 20 | 20 | 16 | 1 | 13 | 13 |
| shared_segments | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| shared_length | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| bends | 0 | 0 | 0 | 0 | 40 | 8 | 18 | 18 |
| edge_length | 334890 | 41156 | 213915 | 213915 | 587990 | 97542 | 356713 | 356713 |
| longest_edge | 58905 | 7658 | 32846 | 32846 | 106710 | 18909 | 45767 | 45767 |
| node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| node_overlap_area | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| edge_node_overlaps | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| label_overlaps | 16 | 1 | 10 | 10 | 30 | 0 | 20 | 20 |
| width | 109100 | 26400 | 31300 | 31300 | 158300 | 29600 | 69200 | 69200 |
| height | 251200 | 30200 | 102200 | 102200 | 219922 | 30400 | 98122 | 98122 |
| area | 7113280000 | 860700000 | 3198860000 | 3198860000 | 9359723000 | 899840000 | 4719440000 | 4719440000 |
| proportion | 2497 | 306 | 970 | 970 | 3517 | 974 | 1207 | 1207 |
| density | 1879 | 415 | 582 | 582 | 1556 | 338 | 484 | 484 |
| layer_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| port_order_violations | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

On the 4 graphs laid out by both engines, how often each does better, lower being better:

| measure | dot better | equal | elk better |
|---|---:|---:|---:|
| crossings | 0 | 3 | 1 |
| bends | 4 | 0 | 0 |
| edge_length | 4 | 0 | 0 |
| area | 4 | 0 | 0 |
