#!/usr/bin/env bash
# Where the time goes (AGENTS.md "Track where the time goes"). One line per
# activity *start* in tmp/timelog.txt; an activity lasts until the next start.
#
#   tools/timelog.sh start <area> [detail...]   begin an activity
#   tools/timelog.sh stop                        end the current one (idle after)
#   tools/timelog.sh report                      minutes per area, and per detail
#   tools/timelog.sh reset                       start a fresh log
#
# Areas are short, stable names, so reports add up across sessions:
#   reading, planning, edit-core, edit-rust, edit-kotlin, edit-std, edit-runtime,
#   edit-tests, edit-docs, build, test-suite, test-targeted, test-std,
#   examples, bench, debug, git, waiting, idle
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
log="$root/tmp/timelog.txt"
mkdir -p "$root/tmp"
case "${1:-report}" in
    start)
        shift
        area="${1:?area}"; shift || true
        printf '%s\t%s\t%s\n' "$(date +%s)" "$area" "$*" >> "$log"
        ;;
    stop)
        printf '%s\tidle\t\n' "$(date +%s)" >> "$log"
        ;;
    reset)
        : > "$log"
        ;;
    report)
        [[ -s "$log" ]] || { echo "no entries in $log"; exit 0; }
        now="$(date +%s)"
        awk -F'\t' -v now="$now" '
            { t[NR]=$1; a[NR]=$2; d[NR]=$3 }
            END {
                for (i = 1; i <= NR; i++) {
                    end = (i < NR) ? t[i+1] : now
                    s = end - t[i]
                    if (a[i] == "idle") continue
                    area[a[i]] += s; total += s
                    key = a[i] " | " d[i]; detail[key] += s
                }
                printf "%-16s %8s %6s\n", "area", "minutes", "share"
                for (k in area) printf "%-16s %8.1f %5.0f%%\n", k, area[k]/60, 100*area[k]/total | "sort -k2 -rn"
                close("sort -k2 -rn")
                printf "%-16s %8.1f\n\n", "total", total/60
                print "by detail (top 15):"
                for (k in detail) printf "%8.1f  %s\n", detail[k]/60, k | "sort -rn | head -15"
            }' "$log"
        ;;
    *) echo "usage: $0 start <area> [detail] | stop | report | reset" >&2; exit 2 ;;
esac
