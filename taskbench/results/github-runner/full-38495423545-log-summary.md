# Machine 2, full-scale run 38495423545: summary from the job log

The run completed (3 h 28 min) but its artefact upload failed on an
invalid artefact name, so the CSVs were lost; this is the `summarize.py
--markdown` output captured from the job log, with timestamps removed.
A corrected run (38513665561) reproduces the CSVs.

```
| A/1M N=100000 | 6.43 [6.43-6.44] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 34.4 [34.4-34.4] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 2.01 [2.01-2.01] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  |  |  |  |
| A/1M N=1000000 | 6.43 [6.43-6.44] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 2.01 [2.01-2.01] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  |  |  |  |
| A/8M N=100000 | 8.45 [8.45-8.45] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 36.5 [36.5-36.5] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 4.03 [4.03-4.03] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  |  |  |  |
| A/8M N=1000000 | 8.45 [8.45-8.46] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 4.03 [4.03-4.03] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  |  |  |  |
| B/128K N=100000 | 4.69 [4.69-4.69] (n=5) | 32.7 [32.7-32.7] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 0.27 [0.27-0.27] (n=5) | 1.75 (CI 1.75-1.75; range 1.75-1.75) | 6.52 (CI 6.52-6.63; range 6.52-6.63) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 6.48 (CI 6.48-6.6; range 6.48-6.6) |
| B/128K N=1000000 | 4.68 [4.68-4.68] (n=5) | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 4.42 [4.42-4.42] (n=5) | 0.27 [0.27-0.27] (n=5) | 1.73 (CI 1.73-1.77; range 1.73-1.77) |  | 1.64 (CI 1.64-1.68; range 1.64-1.68) |  |
| B/1M N=100000 | 6.44 [6.44-6.44] (n=5) | 34.4 [34.4-34.4] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 2.02 [2.02-2.02] (n=5) | 2.41 (CI 2.4-2.41; range 2.4-2.41) | 6.87 (CI 6.87-6.99; range 6.87-6.99) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 6.48 (CI 6.48-6.6; range 6.48-6.6) |
| B/1M N=1000000 | 6.44 [6.44-6.44] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 4.42 [4.42-4.42] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes | 2.02 [2.02-2.02] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes |  |  |  |  |
| B/8M N=100000 | 8.46 [8.46-8.46] (n=5) | 36.6 [36.6-36.6] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 4.03 [4.03-4.03] (n=5) | 3.16 (CI 3.15-3.16; range 3.15-3.16) | 7.29 (CI 7.29-7.42; range 7.29-7.42) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 6.48 (CI 6.48-6.6; range 6.48-6.6) |
| B/8M N=1000000 | 8.45 [8.45-8.45] (n=5) failed: created 696320 of 1000000: memory headroom below 1073741824 bytes | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 4.42 [4.42-4.42] (n=5) failed: created 696320 of 1000000: memory headroom below 1073741824 bytes | 4.03 [4.03-4.03] (n=5) failed: created 696320 of 1000000: memory headroom below 1073741824 bytes |  |  |  |  |
| C/36K N=100000 | 5.50 [5.50-5.50] (n=5) | 35.5 [35.5-35.5] (n=5) |  | 5.42 [5.42-5.42] (n=5) | 0.08 [0.08-0.08] (n=5) | 2.06 (CI 2.05-2.06; range 2.05-2.06) | 7.08 (CI 7.08-7.2; range 7.08-7.2) | 2.03 (CI 2.02-2.03; range 2.02-2.03) | 7.08 (CI 7.08-7.21; range 7.08-7.21) |
| C/36K N=1000000 | 5.50 [5.50-5.50] (n=5) | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 5.42 [5.42-5.42] (n=5) | 0.08 [0.08-0.08] (n=5) | 2.03 (CI 2.03-2.08; range 2.03-2.08) |  | 2.01 (CI 2.01-2.06; range 2.01-2.06) |  |
| C/40K N=100000 | 5.51 [5.51-5.51] (n=5) | 35.5 [35.5-35.5] (n=5) |  | 5.42 [5.42-5.42] (n=5) | 0.09 [0.09-0.09] (n=5) | 2.06 (CI 2.05-2.06; range 2.05-2.06) | 7.08 (CI 7.08-7.21; range 7.08-7.21) | 2.03 (CI 2.02-2.03; range 2.02-2.03) | 7.08 (CI 7.08-7.21; range 7.08-7.21) |
| C/40K N=1000000 | 5.50 [5.50-5.50] (n=5) | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 5.42 [5.42-5.42] (n=5) | 0.09 [0.09-0.09] (n=5) | 2.04 (CI 2.04-2.09; range 2.04-2.09) |  | 2.01 (CI 2.01-2.06; range 2.01-2.06) |  |
| C/48K N=100000 | 5.53 [5.53-5.53] (n=5) | 35.5 [35.5-35.5] (n=5) |  | 5.42 [5.42-5.42] (n=5) | 0.10 [0.10-0.10] (n=5) | 2.07 (CI 2.06-2.07; range 2.06-2.07) | 7.09 (CI 7.08-7.21; range 7.08-7.21) | 2.03 (CI 2.02-2.03; range 2.02-2.03) | 7.08 (CI 7.08-7.21; range 7.08-7.21) |
| C/48K N=1000000 | 5.52 [5.52-5.52] (n=5) | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o |  | 5.42 [5.42-5.42] (n=5) | 0.10 [0.10-0.10] (n=5) | 2.04 (CI 2.04-2.09; range 2.04-2.09) |  | 2.01 (CI 2.01-2.06; range 2.01-2.06) |  |
| D(A)/128K N=100000 | 4.68 [4.68-4.68] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 4.69 [4.69-4.69] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 0.26 [0.26-0.26] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  |  |  |  |
| D(A)/128K N=1000000 | 4.68 [4.68-4.68] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 4.69 [4.69-4.69] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 0.26 [0.26-0.26] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  |  |  |  |
| D(A)/1M N=100000 | 6.43 [6.43-6.43] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 6.44 [6.43-6.44] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 2.01 [2.01-2.01] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  |  |  |  |
| D(A)/1M N=1000000 | 6.43 [6.43-6.44] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 6.45 [6.44-6.45] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 2.01 [2.01-2.01] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  |  |  |  |
| D(A)/8M N=100000 | 8.45 [8.45-8.45] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 8.46 [8.46-8.47] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count | 4.03 [4.03-4.03] (n=5) failed: created 32738 of 100000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_count |  |  |  |  |
| D(A)/8M N=1000000 | 8.45 [8.45-8.45] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 8.46 [8.46-8.46] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  | 4.42 [4.42-4.42] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun | 4.03 [4.03-4.03] (n=5) failed: created 32738 of 1000000: spawn failed: VMA limit reached (65532 mappings, vm.max_map_coun |  |  |  |  |
| D(B)/128K N=100000 | 4.69 [4.69-4.69] (n=5) | 4.69 [4.69-4.69] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 0.27 [0.27-0.27] (n=5) | 1.75 (CI 1.75-1.75; range 1.75-1.75) | 0.936 (CI 0.936-0.952; range 0.936-0.952) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 0.885 (CI 0.885-0.9; range 0.885-0.9) |
| D(B)/128K N=1000000 | 4.68 [4.68-4.68] (n=5) | 4.69 [4.68-4.69] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 0.27 [0.27-0.27] (n=5) | 1.73 (CI 1.73-1.77; range 1.73-1.77) |  | 1.64 (CI 1.64-1.68; range 1.64-1.68) |  |
| D(B)/1M N=100000 | 6.44 [6.44-6.44] (n=5) | 6.45 [6.45-6.45] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 2.02 [2.02-2.02] (n=5) | 2.41 (CI 2.4-2.41; range 2.4-2.41) | 1.29 (CI 1.29-1.31; range 1.29-1.31) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 0.885 (CI 0.885-0.9; range 0.885-0.901) |
| D(B)/1M N=1000000 | 6.44 [6.44-6.44] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes | failed: 962554 of 966656 tasks skipped the excursion: memory headroom below 1073741824 bytes |  | 4.42 [4.42-4.42] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes | 2.02 [2.02-2.02] (n=5) failed: created 966656 of 1000000: memory headroom below 1073741824 bytes |  |  |  |  |
| D(B)/8M N=100000 | 8.46 [8.45-8.46] (n=5) | 8.46 [8.46-8.46] (n=5) |  | 4.42 [4.42-4.42] (n=5) | 4.03 [4.03-4.03] (n=5) | 3.16 (CI 3.15-3.16; range 3.15-3.16) | 1.69 (CI 1.69-1.72; range 1.69-1.72) | 1.66 (CI 1.65-1.66; range 1.65-1.66) | 0.885 (CI 0.885-0.901; range 0.885-0.901) |
| D(B)/8M N=1000000 | 8.45 [8.45-8.45] (n=5) failed: created 733184 of 1000000: memory headroom below 1073741824 bytes | failed: 730079 of 733184 tasks skipped the excursion: memory headroom below 1073741824 bytes |  | 4.42 [4.42-4.42] (n=5) failed: created 733184 of 1000000: memory headroom below 1073741824 bytes | 4.03 [4.03-4.03] (n=5) failed: created 733184 of 1000000: memory headroom below 1073741824 bytes |  |  |  |  |
| E N=100000 | 0.64 [0.63-0.64] (n=5) | 0.64 [0.64-0.64] (n=5) |  | 0.63 [0.63-0.63] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.238 (CI 0.237-0.238; range 0.236-0.238) | 0.128 (CI 0.128-0.131; range 0.128-0.131) | 0.238 (CI 0.237-0.238; range 0.237-0.238) | 0.128 (CI 0.128-0.131; range 0.128-0.131) |
| E N=1000000 | 0.63 [0.63-0.63] (n=5) | 0.64 [0.64-0.64] (n=5) |  | 0.63 [0.63-0.63] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.235 (CI 0.234-0.24; range 0.234-0.241) |  | 0.235 (CI 0.235-0.24; range 0.235-0.24) |  |
| go N=100000 | 2.68 [2.68-2.69] (n=5) | 34.7 [24.4-34.7] (n=5) | 5.01 [4.93-5.02] (n=5) | 2.67 [2.67-2.68] (n=5) | 0.00 [0.00-0.00] (n=5) | 1 (CI 0.996-1; range 0.996-1) | 6.92 (CI 4.87-7.04; range 4.87-7.05) | 1 (CI 0.997-1; range 0.996-1) | 6.91 (CI 4.86-7.03; range 4.86-7.04) |
| go N=1000000 | 2.70 [2.64-2.70] (n=5) | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o | skipped: not run (--skip-excursion): the 32768-byte excursion phase is required only of strategies that release pages o | 2.70 [2.63-2.70] (n=5) | 0.00 [0.00-0.00] (n=5) | 1 (CI 0.977-1.02; range 0.976-1.02) |  | 1 (CI 0.976-1.02; range 0.976-1.02) |  |
| java N=100000 | 2.12 [1.76-2.42] (n=5) | 2.96 [2.59-3.24] (n=5) |  | 2.11 [1.76-2.41] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.791 (CI 0.659-0.904; range 0.657-0.904) | 0.59 (CI 0.518-0.646; range 0.517-0.657) | 0.791 (CI 0.66-0.904; range 0.657-0.904) | 0.591 (CI 0.518-0.646; range 0.517-0.657) |
| java N=1000000 | 1.08 [1.03-1.11] (n=5) | failed: 839851 of 1000000 tasks skipped the excursion: memory headroom below 1073741824 bytes |  | 1.08 [1.03-1.11] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.399 (CI 0.381-0.413; range 0.381-0.422) |  | 0.399 (CI 0.381-0.414; range 0.381-0.422) |  |
| os-threads N=10000 | 9.81 [9.81-9.81] (n=5) failed: created 9435 of 10000: thread spawn: Resource temporarily unavailable (os error 11) | 41.8 [41.8-41.8] (n=5) failed: created 9435 of 10000: thread spawn: Resource temporarily unavailable (os error 11) |  | 9.64 [9.64-9.65] (n=5) failed: created 9435 of 10000: thread spawn: Resource temporarily unavailable (os error 11) | 0.17 [0.16-0.17] (n=5) failed: created 9435 of 10000: thread spawn: Resource temporarily unavailable (os error 11) |  |  |  |  |
| tokio N=100000 | 0.40 [0.40-0.40] (n=5) | 0.40 [0.40-0.41] (n=5) |  | 0.40 [0.40-0.40] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.151 (CI 0.15-0.151; range 0.15-0.151) | 0.0806 (CI 0.0806-0.082; range 0.0806-0.0822) | 0.151 (CI 0.15-0.151; range 0.15-0.151) | 0.0806 (CI 0.0806-0.082; range 0.0806-0.0822) |
| tokio N=1000000 | 0.40 [0.40-0.40] (n=5) | 0.40 [0.40-0.40] (n=5) |  | 0.40 [0.40-0.40] (n=5) | 0.00 [0.00-0.00] (n=5) | 0.148 (CI 0.148-0.151; range 0.148-0.151) |  | 0.148 (CI 0.148-0.151; range 0.148-0.151) |  |

## memory rule (proposal/20, section 2.4) on github-runner: ratio to Go, worst over four conditions

| configuration | 100k shallow | 100k after excursion | 1M shallow | 1M after excursion | verdict on this machine |
|---|---|---|---|---|---|
| A/128K | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| A/1M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| A/8M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | not required (policy 7) | fails |
| B/128K | 1.75 (CI 1.75-1.75; range 1.75-1.75) vs Go | 6.52 (CI 6.52-6.63; range 6.52-6.63) vs Go after GC | 1.73 (CI 1.73-1.77; range 1.73-1.77) vs Go | not required (policy 7) | fails (w = 6.52) |
| B/1M | 2.41 (CI 2.4-2.41; range 2.4-2.41) vs Go | 6.87 (CI 6.87-6.99; range 6.87-6.99) vs Go after GC | not computable: memory guard at 966,656 of 1,000,000 tasks | not required (policy 7) | fails (w = 6.87) |
| B/8M | 3.16 (CI 3.15-3.16; range 3.15-3.16) vs Go | 7.29 (CI 7.29-7.42; range 7.29-7.42) vs Go after GC | not computable: memory guard at 696,320 of 1,000,000 tasks | not required (policy 7) | fails (w = 7.29) |
| C/36K | 2.06 (CI 2.05-2.06; range 2.05-2.06) vs Go | 7.08 (CI 7.08-7.2; range 7.08-7.2) vs Go after GC | 2.03 (CI 2.03-2.08; range 2.03-2.08) vs Go | not required (policy 7) | fails (w = 7.08) |
| C/40K | 2.06 (CI 2.05-2.06; range 2.05-2.06) vs Go | 7.08 (CI 7.08-7.21; range 7.08-7.21) vs Go after GC | 2.04 (CI 2.04-2.09; range 2.04-2.09) vs Go | not required (policy 7) | fails (w = 7.08) |
| C/48K | 2.07 (CI 2.06-2.07; range 2.06-2.07) vs Go | 7.09 (CI 7.08-7.21; range 7.08-7.21) vs Go after GC | 2.04 (CI 2.04-2.09; range 2.04-2.09) vs Go | not required (policy 7) | fails (w = 7.09) |
| D(A)/128K | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(A)/1M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(A)/8M | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 100,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails: VMA limit at 32,738 of 1,000,000 tasks | fails |
| D(B)/128K | 1.75 (CI 1.75-1.75; range 1.75-1.75) vs Go | 0.936 (CI 0.936-0.952; range 0.936-0.952) vs Go after GC | 1.73 (CI 1.73-1.77; range 1.73-1.77) vs Go | 0.935 (CI 0.934-0.951; range 0.934-0.951) vs Go after GC at 100k; 1.73 (CI 1.73-1.78; range 1.73-1.78) vs Go shallow at 1M | passes (w = 1.75) |
| D(B)/1M | 2.41 (CI 2.4-2.41; range 2.4-2.41) vs Go | 1.29 (CI 1.29-1.31; range 1.29-1.31) vs Go after GC | not computable: memory guard at 966,656 of 1,000,000 tasks | not computable: memory guard at 966,656 of 1,000,000 tasks | inconclusive (w = 2.41; a range crosses 4 or a cell did not fit) |
| D(B)/8M | 3.16 (CI 3.15-3.16; range 3.15-3.16) vs Go | 1.69 (CI 1.69-1.72; range 1.69-1.72) vs Go after GC | not computable: memory guard at 733,184 of 1,000,000 tasks | not computable: memory guard at 733,184 of 1,000,000 tasks | inconclusive (w = 3.16; a range crosses 4 or a cell did not fit) |

## spawn-join on github-runner: microseconds per spawn plus join

| configuration | median | p99 | median / Go | p99 / Go |
|---|---|---|---|---|
| A/1M | 11.6 [11.6-11.8] (n=5) | 24.4 [24.0-25.0] (n=5) | 32.1 (CI 31.3-32.7; range 31.2-32.7) | 21.4 (CI 14.1-22.6; range 13.8-23.1) |
| A/1M stack_cache=1024 | 0.29 [0.29-0.31] (n=5) | 0.39 [0.36-0.44] (n=5) | 0.806 (CI 0.784-0.861; range 0.782-0.861) | 0.342 (CI 0.226-0.386; range 0.208-0.408) |
| B/1M | 0.41 [0.41-0.42] (n=5) | 0.79 [0.74-0.82] (n=5) | 1.14 (CI 1.11-1.16; range 1.11-1.16) | 0.694 (CI 0.457-0.732; range 0.428-0.76) |
| B/1M batch=100 | 0.32 [0.32-0.32] (n=5) | 0.47 [0.46-0.54] (n=5) | 0.884 (CI 0.86-0.886; range 0.857-0.886) | 0.412 (CI 0.271-0.476; range 0.264-0.503) |
| B/1M scope_per_round=true | 0.58 [0.56-0.59] (n=5) | 2.63 [2.58-2.65] (n=5) | 1.61 (CI 1.55-1.63; range 1.51-1.63) | 2.31 (CI 1.52-2.44; range 1.49-2.44) |
| C/8K | 0.42 [0.41-0.42] (n=5) | 0.76 [0.75-0.81] (n=5) | 1.16 (CI 1.13-1.17; range 1.11-1.17) | 0.666 (CI 0.439-0.71; range 0.433-0.75) |
| D(A)/1M | 16.1 [12.7-16.1] (n=5) | 31.9 [25.7-34.3] (n=5) | 44.5 (CI 35.2-44.7; range 34.3-44.7) | 28 (CI 18.4-30.2; range 14.8-31.7) |
| D(A)/1M stack_cache=1024 | 1.15 [1.14-1.16] (n=5) | 1.23 [1.21-1.67] (n=5) | 3.19 (CI 3.11-3.22; range 3.08-3.22) | 1.08 (CI 0.711-1.46; range 0.699-1.55) |
| D(B)/1M | 0.43 [0.43-0.44] (n=5) | 0.84 [0.77-0.86] (n=5) | 1.19 (CI 1.16-1.22; range 1.16-1.22) | 0.736 (CI 0.485-0.777; range 0.445-0.796) |
| E | 0.25 [0.25-0.26] (n=5) | 0.30 [0.29-0.36] (n=5) | 0.695 (CI 0.677-0.723; range 0.677-0.723) | 0.264 (CI 0.174-0.319; range 0.168-0.334) |
| go | 0.36 [0.36-0.37] (n=5) | 1.14 [1.08-1.73] (n=5) | 1 (CI 0.973-1.03; range 0.973-1.03) | 1 (CI 0.659-1.52; range 0.624-1.6) |
| java | 0.95 [0.90-1.57] (n=5) | 13.2 [13.0-13.4] (n=5) | 2.64 (CI 2.5-4.36; range 2.43-4.36) | 11.6 (CI 7.64-12.2; range 7.49-12.4) |
| os-threads | 60.1 [59.8-60.7] (n=5) | 149 [144-152] (n=5) | 167 (CI 162-168; range 161-168) | 130 (CI 85.8-137; range 83.2-141) |
| tokio | 0.34 [0.33-0.34] (n=5) | 3.66 [3.50-3.76] (n=5) | 0.942 (CI 0.916-0.942; range 0.892-0.942) | 3.2 (CI 2.11-3.38; range 2.02-3.47) |

## max-tasks on github-runner

| configuration | tasks created | limit | KiB per task at stop | VMAs at stop | held (s) | still parked after the hold | KiB per task after the hold |
|---|---|---|---|---|---|---|---|
| A/1M N=1000000 | 32,738 [32,738-32,738] (n=5) | vma_count | 6.43 [6.43-6.44] (n=5) | 65,531 [65,531-65,531] (n=5) |  |  |  |
| B/1M N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 6.44 [6.44-6.44] (n=5) | 57 [56-58] (n=5) | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 6.44 [6.44-6.44] (n=5) |
| C/8K N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 5.44 [5.44-5.44] (n=5) | 57 [56-57] (n=5) | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 5.44 [5.44-5.44] (n=5) |
| D(A)/1M N=1000000 | 32,738 [32,738-32,738] (n=5) | vma_count | 6.43 [6.43-6.43] (n=5) | 65,531 [65,531-65,531] (n=5) |  |  |  |
| D(B)/1M N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 6.44 [6.44-6.44] (n=5) | 56 [55-58] (n=5) | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 6.44 [6.44-6.44] (n=5) |
| E N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 0.59 [0.59-0.59] (n=5) | 54 [53-55] (n=5) | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0.59 [0.59-0.59] (n=5) |
| go N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 2.63 [2.63-2.63] (n=5) |  | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 2.63 [2.63-2.63] (n=5) |
| java N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 0.98 [0.96-1.01] (n=5) |  | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0.98 [0.96-1.01] (n=5) |
| os-threads N=30000 | 9,435 [9,435-9,438] (n=5) | threads | 10.00 [9.99-10.0] (n=5) | 37,807 [37,807-37,819] (n=5) |  |  |  |
| tokio N=1000000 | 1,000,000 [1,000,000-1,000,000] (n=5) | cap | 0.39 [0.39-0.39] (n=5) | 51 [51-52] (n=5) | 60.0 [60.0-60.0] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0.39 [0.39-0.39] (n=5) |

## echo on github-runner: round trip in ms, requests per second

| configuration | p50 | p99 | p99.9 | req/s | p99 / Go |
|---|---|---|---|---|---|
| A/1M connections=1000 | 12.3 [12.0-12.3] (n=5) | 14.7 [14.4-14.9] (n=5) | 17.8 [17.0-23.2] (n=5) | 80,753 [79,984-82,129] (n=5) | 0.855 (CI 0.797-0.87; range 0.78-0.879) |
| A/1M connections=10000 | 201 [199-201] (n=5) | 219 [213-223] (n=5) | 235 [223-260] (n=5) | 49,768 [49,480-50,085] (n=5) | 1.01 (CI 0.959-1.03; range 0.948-1.05) |
| B/1M connections=1000 | 12.3 [11.9-12.5] (n=5) | 14.8 [14.6-15.3] (n=5) | 20.1 [17.5-27.0] (n=5) | 80,433 [79,534-83,133] (n=5) | 0.863 (CI 0.805-0.889; range 0.794-0.902) |
| B/1M connections=10000 | 201 [201-206] (n=5) | 216 [213-225] (n=5) | 223 [222-262] (n=5) | 49,692 [48,593-49,737] (n=5) | 1 (CI 0.958-1.04; range 0.948-1.06) |
| C/8K connections=1000 | 12.1 [11.9-12.3] (n=5) | 14.5 [14.1-16.3] (n=5) | 19.9 [16.6-20.7] (n=5) | 81,978 [79,869-83,120] (n=5) | 0.846 (CI 0.788-0.947; range 0.765-0.962) |
| C/8K connections=10000 | 200 [199-203] (n=5) | 213 [209-219] (n=5) | 226 [220-260] (n=5) | 50,041 [49,161-50,177] (n=5) | 0.988 (CI 0.942-1.02; range 0.932-1.03) |
| D(A)/1M connections=1000 | 16.6 [16.1-17.0] (n=5) | 19.8 [19.4-20.1] (n=5) | 21.5 [20.6-26.2] (n=5) | 59,461 [58,370-61,410] (n=5) | 1.15 (CI 1.07-1.17; range 1.05-1.19) |
| D(A)/1M connections=10000 | 236 [235-238] (n=5) | 248 [243-295] (n=5) | 284 [246-319] (n=5) | 42,324 [41,851-42,366] (n=5) | 1.15 (CI 1.1-1.37; range 1.08-1.39) |
| D(B)/1M connections=1000 | 12.2 [12.2-12.3] (n=5) | 14.7 [14.5-15.2] (n=5) | 20.5 [20.0-22.0] (n=5) | 80,607 [80,097-80,936] (n=5) | 0.857 (CI 0.799-0.884; range 0.79-0.897) |
| D(B)/1M connections=10000 | 199 [198-201] (n=5) | 220 [212-254] (n=5) | 255 [222-338] (n=5) | 50,035 [49,654-50,623] (n=5) | 1.02 (CI 0.973-1.18; range 0.946-1.2) |
| E connections=1000 | 12.1 [12.0-12.3] (n=5) | 14.6 [14.3-14.7] (n=5) | 18.4 [16.7-20.6] (n=5) | 81,672 [80,428-82,177] (n=5) | 0.851 (CI 0.793-0.867; range 0.776-0.87) |
| E connections=10000 | 198 [197-200] (n=5) | 212 [209-223] (n=5) | 221 [219-263] (n=5) | 50,291 [50,038-50,543] (n=5) | 0.981 (CI 0.942-1.04; range 0.932-1.05) |
| go connections=1000 | 13.5 [13.3-14.0] (n=5) | 17.2 [16.9-18.4] (n=5) | 19.7 [18.7-25.0] (n=5) | 74,011 [71,046-74,740] (n=5) | 1 (CI 0.932-1.07; range 0.918-1.09) |
| go connections=10000 | 205 [203-205] (n=5) | 216 [212-224] (n=5) | 222 [219-258] (n=5) | 48,787 [48,566-49,216] (n=5) | 1 (CI 0.959-1.04; range 0.946-1.06) |
| java connections=1000 | 27.2 [27.0-29.0] (n=5) | 37.6 [37.0-39.6] (n=5) | 43.6 [42.9-51.0] (n=5) | 36,643 [34,969-37,010] (n=5) | 2.19 (CI 2.04-2.3; range 2.01-2.34) |
| java connections=10000 | 344 [339-350] (n=5) | 558 [516-689] (n=5) | 664 [548-727] (n=5) | 28,790 [25,175-29,022] (n=5) | 2.58 (CI 2.39-3.19; range 2.3-3.25) |
| tokio connections=1000 | 4.09 [4.08-4.11] (n=5) | 54.8 [54.2-55.0] (n=5) | 70.5 [69.6-71.0] (n=5) | 90,218 [89,871-90,709] (n=5) | 3.19 (CI 2.98-3.25; range 2.94-3.25) |
| tokio connections=10000 | 5.32 [5.26-5.35] (n=5) | 671 [667-681] (n=5) | 714 [705-747] (n=5) | 72,732 [72,441-73,044] (n=5) | 3.11 (CI 2.99-3.17; range 2.98-3.21) |

## idle-connections on github-runner: KiB per idle connection

| configuration | RSS + page tables | RSS | total / Go |
|---|---|---|---|
| A/1M connections=32700 | 10.5 [10.5-10.5] (n=5) | 8.53 [8.53-8.53] (n=5) | 1.96 (CI 1.93-1.99; range 1.93-1.99) |
| B/1M connections=32700 | 10.6 [10.6-10.6] (n=5) | 8.55 [8.54-8.55] (n=5) | 1.96 (CI 1.94-1.99; range 1.94-1.99) |
| C/8K connections=32700 | 8.58 [8.58-8.58] (n=5) | 8.54 [8.54-8.55] (n=5) | 1.59 (CI 1.57-1.62; range 1.57-1.62) |
| D(A)/1M connections=32700 | 10.6 [10.5-10.6] (n=5) | 8.53 [8.53-8.53] (n=5) | 1.96 (CI 1.93-1.99; range 1.93-1.99) |
| D(B)/1M connections=32700 | 10.6 [10.6-10.6] (n=5) | 8.54 [8.54-8.55] (n=5) | 1.96 (CI 1.94-1.99; range 1.94-1.99) |
| E connections=32700 | 4.96 [4.96-4.96] (n=5) | 4.95 [4.95-4.95] (n=5) | 0.921 (CI 0.909-0.935; range 0.909-0.935) |
| go connections=32700 | 5.38 [5.31-5.46] (n=5) | 5.37 [5.29-5.44] (n=5) | 1 (CI 0.986-1.01; range 0.973-1.03) |
| java connections=32700 | 8.48 [8.21-8.82] (n=5) | 8.46 [8.19-8.80] (n=5) | 1.58 (CI 1.53-1.64; range 1.51-1.66) |
| tokio connections=32700 | 4.78 [4.78-4.79] (n=5) | 4.77 [4.77-4.78] (n=5) | 0.889 (CI 0.877-0.902; range 0.877-0.902) |

## deep-recursion on github-runner

| configuration | 10k parse | parse depth | 1M naive | naive depth | naive async (E, tokio) | drop guards skipped | framed (256-byte frames, drop code) | max reserve bytes used | healthy after |
|---|---|---|---|---|---|---|---|---|---|
| A/128K | stack_exhausted | 354 [354-354] (n=5) | stack_exhausted | 2,722 [2,722-2,722] (n=5) |  | 11 [11-11] (n=5) | stack_exhausted, depth 429 [429-429] (n=5), guards skipped 429 [429-429] (n=5) |  | 1 [1-1] (n=5) |
| A/1M | stack_exhausted | 2,848 [2,848-2,848] (n=5) | stack_exhausted | 21,836 [21,836-21,836] (n=5) |  | 86 [86-86] (n=5) | stack_exhausted, depth 3,447 [3,447-3,447] (n=5), guards skipped 3,448 [3,448-3,448] (n=5) |  | 1 [1-1] (n=5) |
| A/8M | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 174,754 [174,754-174,754] (n=5) |  | 683 [683-683] (n=5) | stack_exhausted, depth 27,592 [27,592-27,592] (n=5), guards skipped 27,592 [27,592-27,592] (n=5) |  | 1 [1-1] (n=5) |
| B/128K | stack_exhausted | 363 [363-363] (n=5) | stack_exhausted | 2,785 [2,785-2,785] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 439 [439-439] (n=5), guards skipped 0 [0-0] (n=5) | parse 1880 / naive 1800 / framed 2088 | 1 [1-1] (n=5) |
| B/1M | stack_exhausted | 2,856 [2,856-2,856] (n=5) | stack_exhausted | 21,900 [21,900-21,900] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 3,457 [3,457-3,457] (n=5), guards skipped 0 [0-0] (n=5) | parse 1800 / naive 1816 / framed 2056 | 1 [1-1] (n=5) |
| B/8M | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 174,817 [174,817-174,817] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 27,601 [27,601-27,601] (n=5), guards skipped 0 [0-0] (n=5) | parse 0 / naive 1800 / framed 1800 | 1 [1-1] (n=5) |
| C/16K | stack_exhausted | 43 [43-43] (n=5) | stack_exhausted | 332 [332-332] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 51 [51-51] (n=5), guards skipped 0 [0-0] (n=5) | parse 1880 / naive 1816 / framed 1896 | 1 [1-1] (n=5) |
| C/2K | stack_exhausted | 4 [4-4] (n=5) | stack_exhausted | 33 [33-33] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 4 [4-4] (n=5), guards skipped 0 [0-0] (n=5) | parse 1864 / naive 1800 / framed 1944 | 1 [1-1] (n=5) |
| C/8K | stack_exhausted | 20 [20-20] (n=5) | stack_exhausted | 161 [161-161] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 24 [24-24] (n=5), guards skipped 0 [0-0] (n=5) | parse 1880 / naive 1800 / framed 1880 | 1 [1-1] (n=5) |
| D(A)/128K | stack_exhausted | 354 [354-354] (n=5) | stack_exhausted | 2,722 [2,722-2,722] (n=5) |  | 11 [11-11] (n=5) | stack_exhausted, depth 429 [429-429] (n=5), guards skipped 429 [429-429] (n=5) |  | 1 [1-1] (n=5) |
| D(A)/1M | stack_exhausted | 2,848 [2,848-2,848] (n=5) | stack_exhausted | 21,836 [21,836-21,836] (n=5) |  | 86 [86-86] (n=5) | stack_exhausted, depth 3,447 [3,447-3,447] (n=5), guards skipped 3,448 [3,448-3,448] (n=5) |  | 1 [1-1] (n=5) |
| D(A)/8M | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 174,754 [174,754-174,754] (n=5) |  | 683 [683-683] (n=5) | stack_exhausted, depth 27,592 [27,592-27,592] (n=5), guards skipped 27,592 [27,592-27,592] (n=5) |  | 1 [1-1] (n=5) |
| D(B)/128K | stack_exhausted | 363 [363-363] (n=5) | stack_exhausted | 2,785 [2,785-2,785] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 439 [439-439] (n=5), guards skipped 0 [0-0] (n=5) | parse 1880 / naive 1800 / framed 2088 | 1 [1-1] (n=5) |
| D(B)/1M | stack_exhausted | 2,856 [2,856-2,856] (n=5) | stack_exhausted | 21,900 [21,900-21,900] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 3,457 [3,457-3,457] (n=5), guards skipped 0 [0-0] (n=5) | parse 1800 / naive 1816 / framed 2056 | 1 [1-1] (n=5) |
| D(B)/8M | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 174,817 [174,817-174,817] (n=5) |  | 0 [0-0] (n=5) | stack_exhausted, depth 27,601 [27,601-27,601] (n=5), guards skipped 0 [0-0] (n=5) | parse 0 / naive 1800 / framed 1800 | 1 [1-1] (n=5) |
| E | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 173,279 [173,279-173,279] (n=5) | stack_exhausted | 0 [0-0] (n=5) |  |  | 1 [1-1] (n=5) |
| tokio | completed | 10,000 [10,000-10,000] (n=5) | stack_exhausted | 173,273 [173,273-173,273] (n=5) | stack_exhausted | 0 [0-0] (n=5) |  |  | 1 [1-1] (n=5) |

## starvation on github-runner: probe lateness in microseconds (Go baseline: default preemption)

| configuration | slice between yields | p99 alone | p50 loaded | p99 loaded | max loaded | wake-ups | p99 / Go | compute iterations/s |
|---|---|---|---|---|---|---|---|---|
| A/1M | 19.1 [19.0-19.3] (n=5) | 83.5 [78.3-88.1] (n=5) | 7.34 [7.01-7.92] (n=5) | 28.6 [28.0-28.9] (n=5) | 75.1 [73.7-390] (n=5) | 10,903 [10,899-10,906] (n=5) | 0.000728 (CI 0.000714-0.000736; range 0.000714-0.000736) | 1,056,160,008 [1,052,444,244-1,056,718,213] (n=5) |
| A/1M yield_every=100000 | 192 [190-194] (n=5) | 90.9 [82.4-121] (n=5) | 123 [123-123] (n=5) | 308 [308-309] (n=5) | 360 [341-924] (n=5) | 10,032 [9,997-10,053] (n=5) | 0.00784 (CI 0.00783-0.00785; range 0.00783-0.00786) | 1,062,047,159 [1,057,833,315-1,062,083,455] (n=5) |
| B/1M | 19.0 [19.0-19.4] (n=5) | 107 [97.1-113] (n=5) | 6.61 [6.25-7.67] (n=5) | 28.4 [27.0-29.5] (n=5) | 77.0 [73.5-123] (n=5) | 10,908 [10,899-10,910] (n=5) | 0.000723 (CI 0.000687-0.00075; range 0.000687-0.00075) | 1,052,215,219 [1,051,593,346-1,052,351,031] (n=5) |
| B/1M yield_every=0 | 192 [189-192] (n=5) | 229 [103-369] (n=5) | 10,999,260 [39.3-11,000,313] (n=5) | 10,999,260 [10,999,134-11,000,313] (n=5) | 10,999,260 [10,999,134-11,000,313] (n=5) | 1 [1-2] (n=5) | 280 (CI 280-280; range 280-280) | 1,063,272,695 [1,059,770,732-1,064,781,165] (n=5) |
| B/1M yield_every=1000 | 1.92 [1.91-1.93] (n=5) | 111 [88.8-214] (n=5) | 1.02 [0.95-1.11] (n=5) | 2.99 [2.87-3.09] (n=5) | 78.4 [67.6-341] (n=5) | 10,989 [10,988-10,989] (n=5) | 7.61e-05 (CI 7.32e-05-7.86e-05; range 7.31e-05-7.86e-05) | 1,002,469,676 [997,183,710-1,003,838,087] (n=5) |
| B/1M yield_every=100000 | 191 [189-193] (n=5) | 95.2 [91.4-137] (n=5) | 123 [123-123] (n=5) | 308 [308-308] (n=5) | 411 [332-1,577] (n=5) | 10,022 [10,007-10,050] (n=5) | 0.00783 (CI 0.00783-0.00784; range 0.00782-0.00784) | 1,060,274,746 [1,058,146,366-1,061,101,088] (n=5) |
| C/8K | 19.2 [19.1-19.4] (n=5) | 101 [97.3-122] (n=5) | 7.08 [6.20-7.36] (n=5) | 28.5 [26.0-28.9] (n=5) | 75.9 [54.1-78.9] (n=5) | 10,904 [10,902-10,910] (n=5) | 0.000725 (CI 0.000663-0.000737; range 0.000663-0.000737) | 1,056,163,379 [1,051,944,604-1,056,703,905] (n=5) |
| C/8K yield_every=100000 | 191 [189-194] (n=5) | 101 [84.5-134] (n=5) | 123 [123-123] (n=5) | 308 [307-308] (n=5) | 342 [330-392] (n=5) | 10,026 [10,012-10,036] (n=5) | 0.00784 (CI 0.00783-0.00785; range 0.00782-0.00785) | 1,062,009,511 [1,056,769,620-1,063,059,418] (n=5) |
| D(A)/1M | 19.2 [19.1-19.3] (n=5) | 91.7 [85.8-102] (n=5) | 4.92 [3.56-5.53] (n=5) | 25.6 [22.9-27.1] (n=5) | 75.2 [54.6-161] (n=5) | 10,924 [10,916-10,940] (n=5) | 0.000653 (CI 0.000583-0.000689; range 0.000582-0.000689) | 1,054,276,688 [1,047,937,302-1,056,811,959] (n=5) |
| D(A)/1M yield_every=100000 | 192 [191-193] (n=5) | 108 [88.9-539] (n=5) | 124 [124-125] (n=5) | 290 [260-310] (n=5) | 396 [346-442] (n=5) | 10,033 [9,988-10,048] (n=5) | 0.00737 (CI 0.00662-0.00788; range 0.00661-0.00788) | 1,057,345,781 [1,053,412,931-1,059,785,852] (n=5) |
| D(B)/1M | 19.2 [19.0-19.2] (n=5) | 95.1 [92.2-117] (n=5) | 7.12 [6.48-8.39] (n=5) | 28.8 [27.4-29.0] (n=5) | 85.1 [77.9-3,708] (n=5) | 10,902 [10,893-10,908] (n=5) | 0.000733 (CI 0.000698-0.000738; range 0.000698-0.000739) | 1,055,245,280 [1,043,119,193-1,056,638,206] (n=5) |
| D(B)/1M yield_every=100000 | 192 [190-192] (n=5) | 111 [90.1-511] (n=5) | 123 [123-123] (n=5) | 308 [308-308] (n=5) | 345 [339-645] (n=5) | 10,035 [10,029-10,045] (n=5) | 0.00784 (CI 0.00783-0.00784; range 0.00783-0.00784) | 1,061,834,942 [1,057,665,676-1,062,728,425] (n=5) |
| E | 19.2 [19.1-19.3] (n=5) | 85.2 [79.6-205] (n=5) | 9.29 [7.92-11.0] (n=5) | 18.7 [18.6-19.8] (n=5) | 76.9 [51.5-2,783] (n=5) | 10,902 [10,895-10,908] (n=5) | 0.000476 (CI 0.000472-0.000504; range 0.000472-0.000504) | 1,059,008,678 [1,054,787,540-1,059,540,133] (n=5) |
| E yield_every=100000 | 192 [190-193] (n=5) | 133 [84.6-202] (n=5) | 122 [122-122] (n=5) | 147 [146-155] (n=5) | 258 [223-277] (n=5) | 10,072 [10,036-10,102] (n=5) | 0.00375 (CI 0.00372-0.00396; range 0.00372-0.00396) | 1,062,067,563 [1,057,354,380-1,063,224,389] (n=5) |
| go yield_every=0 |  | 109 [97.0-125] (n=5) | 19,140 [19,137-19,145] (n=5) | 39,282 [39,272-39,303] (n=5) | 39,364 [39,318-42,683] (n=5) | 537 [536-538] (n=5) | 1 (CI 0.999-1; range 0.999-1) | 998,635,455 [994,615,527-999,778,362] (n=5) |
| java |  | 525 [417-685] (n=5) | 90.6 [84.8-200] (n=5) | 1,177 [997-1,887] (n=5) | 127,622 [12,773-443,768] (n=5) | 9,255 [8,403-9,743] (n=5) | 0.03 (CI 0.0254-0.048; range 0.0254-0.0481) | 972,940,413 [968,315,802-977,015,009] (n=5) |
| java yield_every=0 |  | 479 [288-928] (n=5) | 11,002,130 [11,001,251-11,004,866] (n=5) | 11,002,130 [11,001,251-11,004,866] (n=5) | 11,002,130 [11,001,251-11,004,866] (n=5) | 1 [1-1] (n=5) | 280 (CI 280-280; range 280-280) | 1,061,278,915 [1,056,714,238-1,061,693,732] (n=5) |
| java yield_every=100000 |  | 452 [325-715] (n=5) | 1,289 [163-1,307] (n=5) | 3,042 [1,609-3,199] (n=5) | 101,961 [6,946-417,612] (n=5) | 4,640 [4,512-9,144] (n=5) | 0.0774 (CI 0.0409-0.0814; range 0.0409-0.0815) | 1,029,558,492 [1,019,831,866-1,034,488,224] (n=5) |
| os-threads | 19.2 [18.8-19.9] (n=5) | 96.7 [90.2-118] (n=5) | 63.5 [63.5-63.6] (n=5) | 2,000 [1,999-2,000] (n=5) | 4,327 [2,322-4,697] (n=5) | 9,591 [9,544-9,650] (n=5) | 0.0509 (CI 0.0509-0.0509; range 0.0509-0.0509) | 1,055,928,079 [1,051,894,647-1,056,804,618] (n=5) |
| tokio | 19.3 [19.2-19.3] (n=5) | 1,138 [1,135-1,144] (n=5) | 1,000 [1,000-1,001] (n=5) | 1,068 [1,066-1,069] (n=5) | 1,498 [1,352-1,757] (n=5) | 5,501 [5,501-5,501] (n=5) | 0.0272 (CI 0.0271-0.0272; range 0.0271-0.0272) | 1,043,254,889 [1,040,517,376-1,044,394,307] (n=5) |
| tokio yield_every=100000 | 191 [189-193] (n=5) | 1,145 [1,141-1,172] (n=5) | 966 [962-969] (n=5) | 1,635 [1,623-1,637] (n=5) | 2,309 [1,835-2,770] (n=5) | 5,496 [5,491-5,497] (n=5) | 0.0416 (CI 0.0413-0.0417; range 0.0413-0.0417) | 1,059,752,037 [1,055,828,523-1,060,738,845] (n=5) |

## probe-overhead on github-runner: percent over the same code without probes

| configuration | fib split | calls split | tree split | loop (back-edge check) | calls arg | calls tls |
|---|---|---|---|---|---|---|
| A/1M | 30.5 [30.3-30.5] (n=5) | 0.0 [-0.2-0.1] (n=5) | 19.9 [3.9-24.1] (n=5) | 0.0 [0.0-0.0] (n=5) | 0.0 [-0.2-0.1] (n=5) | 16.7 [16.6-17.0] (n=5) |
| B/1M | 30.4 [30.0-30.5] (n=5) | 0.1 [0.0-0.1] (n=5) | 19.7 [6.3-20.9] (n=5) | 0.0 [-0.1-0.0] (n=5) | 0.1 [0.0-0.2] (n=5) | 16.9 [16.8-17.0] (n=5) |
| C/8K | 30.4 [30.3-35.7] (n=5) | 0.0 [-0.2-0.1] (n=5) | 21.2 [20.4-21.9] (n=5) | 0.0 [0.0-0.0] (n=5) | 0.0 [-0.2-0.0] (n=5) | 16.8 [16.7-17.0] (n=5) |
| D(B)/1M | 30.5 [30.2-30.6] (n=5) | 0.1 [0.0-0.1] (n=5) | 19.1 [17.3-20.9] (n=5) | 0.0 [0.0-0.0] (n=5) | 0.0 [0.0-0.1] (n=5) | 16.7 [16.6-16.9] (n=5) |
| E | 30.5 [30.3-31.5] (n=5) | 0.0 [0.0-0.1] (n=5) | 5.9 [3.9-12.4] (n=5) | 0.0 [0.0-0.0] (n=5) | 0.0 [0.0-0.0] (n=5) | 16.8 [16.8-17.1] (n=5) |

## cancel-uring on github-runner (per run; buffer writes after return summed over runs)

| configuration | reads | cancels submitted | ended -ECANCELED | cancel ok | -ENOENT | -EALREADY | completed after failed cancel | completed with the late data | buffer bytes written after return | cancel to return p50 / p99 (ms) |
|---|---|---|---|---|---|---|---|---|---|---|
| A/1M | 1,000,000 [1,000,000-1,000,000] (n=5) | 728,297 [719,876-729,338] (n=5) | 286,367 [280,663-295,027] (n=5) | 286,362 [280,646-295,026] (n=5) | 442,938 [433,269-445,760] (n=5) | 15 [2-19] (n=5) | 42,191 [32,570-50,191] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 20.1 [19.8-22.0] (n=5) / 36.3 [34.7-36.9] (n=5) |
| A/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 28.3 [25.9-30.4] (n=5) / 46.0 [41.1-48.3] (n=5) |
| B/1M | 1,000,000 [1,000,000-1,000,000] (n=5) | 688,881 [668,711-691,094] (n=5) | 272,277 [270,312-279,603] (n=5) | 272,252 [270,287-279,591] (n=5) | 411,489 [396,436-418,568] (n=5) | 23 [14-31] (n=5) | 48,256 [47,091-65,364] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 17.6 [14.4-18.2] (n=5) / 37.9 [33.6-38.7] (n=5) |
| B/1M break_contract=true | 250,000 [250,000-250,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 38769152 max, 192462016 total | 1.73 [1.56-1.94] (n=5) / 7.96 [6.87-9.60] (n=5) |
| B/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 27.9 [27.6-29.8] (n=5) / 45.7 [45.1-49.1] (n=5) |
| B/1M source=file | 1,000,000 [1,000,000-1,000,000] (n=5) | 485,991 [484,278-486,204] (n=5) | 0 [0-6] (n=5) | 0 [0-6] (n=5) | 485,991 [484,271-486,204] (n=5) | 0 [0-1] (n=5) | 370,537 [361,782-384,838] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 808 [790-808] (n=5) / 849 [845-868] (n=5) |
| B/1M source=file-buffered | 1,000,000 [1,000,000-1,000,000] (n=5) | 471,428 [470,272-472,845] (n=5) | 158,964 [132,272-180,208] (n=5) | 158,959 [132,261-180,205] (n=5) | 307,916 [284,085-333,602] (n=5) | 5,982 [5,807-7,121] (n=5) | 196,419 [176,082-209,732] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 657 [639-688] (n=5) / 1,105 [1,099-1,188] (n=5) |
| C/8K | 1,000,000 [1,000,000-1,000,000] (n=5) | 692,329 [671,017-700,012] (n=5) | 275,628 [272,055-277,750] (n=5) | 275,614 [272,045-277,735] (n=5) | 416,580 [393,263-425,172] (n=5) | 17 [13-19] (n=5) | 49,209 [45,698-60,368] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 16.5 [14.5-17.6] (n=5) / 39.7 [36.6-41.0] (n=5) |
| C/8K pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 27.8 [27.0-30.2] (n=5) / 46.1 [44.6-50.3] (n=5) |
| D(A)/1M | 1,000,000 [1,000,000-1,000,000] (n=5) | 680,984 [677,275-690,265] (n=5) | 273,105 [272,603-279,387] (n=5) | 273,094 [272,588-279,377] (n=5) | 407,966 [404,169-416,638] (n=5) | 14 [7-17] (n=5) | 155,054 [151,774-163,793] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 33.3 [32.1-34.7] (n=5) / 57.0 [55.3-57.5] (n=5) |
| D(A)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 41.6 [41.2-45.6] (n=5) / 69.8 [68.1-93.4] (n=5) |
| D(B)/1M | 1,000,000 [1,000,000-1,000,000] (n=5) | 716,016 [709,551-721,882] (n=5) | 278,697 [273,989-285,497] (n=5) | 278,690 [273,980-285,487] (n=5) | 436,382 [433,317-442,412] (n=5) | 13 [10-24] (n=5) | 51,181 [42,704-62,527] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 20.2 [18.3-20.9] (n=5) / 36.9 [36.5-38.2] (n=5) |
| D(B)/1M pipe_mix=cancel | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 1,000,000 [1,000,000-1,000,000] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 [0-0] (n=5) | 0 max, 0 total | 29.0 [26.9-29.8] (n=5) / 46.2 [44.1-50.3] (n=5) |

## Machine load during the runs (load.csv)

| results | benchmark | quiet below (CPUs) | processes kept | attempts measured again | gate timeouts | kept although busy | others' CPUs while kept: median | max | steal (CPUs): median | gate wait (s): total / longest |
|---|---|---|---|---|---|---|---|---|---|---|
| full-claude/beyond-the-box | cancel-uring | off | 65 | 0 | 0 | 0 | 0.009 | 0.109 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | deep-recursion | off | 85 | 0 | 0 | 0 | 0.000 | 0.720 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | echo | off | 90 | 0 | 0 | 0 | 0.015 | 0.106 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | idle-connections | off | 45 | 0 | 0 | 0 | 0.007 | 0.013 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | idle-memory | off | 195 | 0 | 0 | 0 | 0.008 | 0.202 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | max-tasks | off | 50 | 0 | 0 | 0 | 0.004 | 0.070 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | probe-overhead | off | 25 | 0 | 0 | 0 | 0.010 | 0.025 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | spawn-join | off | 70 | 0 | 0 | 0 | 0.030 | 1.160 | 0.000 | 0 / 0 |
| full-claude/beyond-the-box | starvation | off | 105 | 0 | 0 | 0 | 0.008 | 0.029 | 0.000 | 0 / 0 |

process_exit rows (crashes): 0
rows: 9980; processes (run ids): 730; non-finite values: 0
##[group]Run actions/upload-artifact@v4
with:
  name: taskbench-full-claude/beyond-the-box
  path: taskbench/results/github-runner/
  if-no-files-found: warn
  compression-level: 6
  overwrite: false
  include-hidden-files: false
env:
  JAVA_HOME: /opt/hostedtoolcache/Java_Temurin-Hotspot_jdk/21.0.12-1/x64
  JAVA_HOME_21_X64: /opt/hostedtoolcache/Java_Temurin-Hotspot_jdk/21.0.12-1/x64
##[endgroup]
(node:257141) [DEP0040] DeprecationWarning: The `punycode` module is deprecated. Please use a userland alternative instead.
(Use `node --trace-deprecation ...` to show where the warning was created)
With the provided path, there will be 27 files uploaded
##[error]The artifact name is not valid: taskbench-full-claude/beyond-the-box. Contains the following character:  Forward slash /
          
Invalid characters include:  Double quote ", Colon :, Less than <, Greater than >, Vertical bar |, Asterisk *, Question mark ?, Carriage return \r, Line feed \n, Backslash \, Forward slash /
          
These characters are not allowed in the artifact name due to limitations with certain file systems such as NTFS. To maintain file system agnostic behavior, these characters are intentionally not allowed to prevent potential problems with downloads on different file systems.
Post job cleanup.
(node:257153) [DEP0040] DeprecationWarning: The `punycode` module is deprecated. Please use a userland alternative instead.
(Use `node --trace-deprecation ...` to show where the warning was created)
Post job cleanup.
[command]/usr/bin/git version
git version 2.55.0
Temporarily overriding HOME='/home/runner/work/_temp/79f0d083-1e9d-43c9-891a-e118d8367489' before making global git config changes
Adding repository directory to the temporary git global config as a safe directory
[command]/usr/bin/git config --global --add safe.directory /home/runner/work/sisu/sisu
[command]/usr/bin/git config --local --name-only --get-regexp core\.sshCommand
[command]/usr/bin/git submodule foreach --recursive sh -c "git config --local --name-only --get-regexp 'core\.sshCommand' && git config --local --unset-all 'core.sshCommand' || :"
[command]/usr/bin/git config --local --name-only --get-regexp http\.https\:\/\/github\.com\/\.extraheader
http.https://github.com/.extraheader
[command]/usr/bin/git config --local --unset-all http.https://github.com/.extraheader
[command]/usr/bin/git submodule foreach --recursive sh -c "git config --local --name-only --get-regexp 'http\.https\:\/\/github\.com\/\.extraheader' && git config --local --unset-all 'http.https://github.com/.extraheader' || :"
[command]/usr/bin/git config --local --name-only --get-regexp ^includeIf\.gitdir:
[command]/usr/bin/git submodule foreach --recursive git config --local --show-origin --name-only --get-regexp remote.origin.url
Cleaning up orphan processes
##[warning]Node.js 20 is deprecated. The following actions target Node.js 20 but are being forced to run on Node.js 24: actions/checkout@v4, actions/setup-go@v5, actions/setup-java@v4, actions/upload-artifact@v4. For more information see: https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/
```
