# SoaringSpot data

Findings about the data that SoaringSpot publishes for each competition day.

The findings were checked against about 75 daily results pages and 16 task pages from these competitions, named here by their SoaringSpot URL slug:

- `39th-fai-world-gliding-championships-tabor-2025` (Tábor)
- `23rd-fai-european-gliding-championships-bekescsaba-2025` (Békéscsaba)
- `13th-fai-womens-world-gliding-championship-zbraslavice-2025` (Zbraslavice)
- `7th-fai-junior-european-gliding-championships-prievidza-2025` (Prievidza)
- `interservices-regional-gliding-comp-2025-middle-wallop-2025` (Middle Wallop)
- `saga-state-gliding-championships-2025` (SAGA)
- `il-nat-megiddo-2026` (Megiddo)
- `68-campeonato-brasileiro-de-voo-em-planadores-itapolis-2026` (Itápolis)
- `qld-state-gliding-competition-2026` (QLD)
- `sanats2026` (SANATS)

## Task page

The task page (`/tasks/{class}/task-{n}-on-{date}`) embeds the complete task as a JSON argument of `Map.SoaringSpot.taskNormalize(...)`. The top-level keys are `task_date`, `task_number`, `task_name`, `task_type` (`polygon` or `assigned_area`), `task_value`, `task_distance`, `task_distance_min`, `task_distance_max`, `task_duration` (minimum task time in seconds, `0` for racing tasks), `no_start`, `start_on_entry`, `qnh`, `result_status`, and `task_points`. In a larger sample of 124 tasks, `task_name` was missing from 14 tasks and `qnh` from 46. All other keys were always present.

Each task point has `name`, `type` (`start`, `point`, `finish`), `latitude` and `longitude` in radians, `elevation`, `distance`, `course_in`, `course_out`, `point_index`, `multiple_start`, and the observation zone fields `oz_type`, `oz_radius1`, `oz_radius2`, `oz_angle1`, `oz_angle2`, `oz_angle12` (angles in radians), `oz_line`, `oz_move`, `oz_reduce`, and `oz_max_altitude`.

The task notes on the same page are free text. They often contain rules that affect scoring, such as the minimum finish altitude, the pre-start altitude limit, and PEV start settings.

### Start opening time

`no_start` is the time the start line opened, in competition local time. In all 7 sampled tasks that set it, no pilot started before it. The value is often entered after the fact, for example when the task notes say the line opens 30 minutes after the last launch.

A time of `00:00:00` means the scorer never entered an opening time. That was the case in 9 of 16 sampled tasks, including FAI championships.

### Leg distances

The `distance` of each point is the length of the leg arriving at that point. The `distance` of the start point is the closing leg from the finish back to the start and is not part of the task. On the Tábor Club class task of 2025-06-19, `task_distance` equals the sum of the leg distances from the first turnpoint to the finish.

`oz_reduce` subtracts the zone radius from the leg. On the Tábor Club class task of 2025-06-19, the finish ring has a 5 km radius with `oz_reduce` set, and the published finish leg is 5000.7 m shorter than the distance between the two point centres.

### Earth model

The Earth model differs between competitions and is not recorded in the task data. Comparing the published leg distances with distances computed from the coordinates gives:

| model | competitions |
|---|---|
| sphere, radius about 6370.8 km | Tábor, Zbraslavice, Prievidza |
| WGS84 ellipsoid | Békéscsaba, Middle Wallop, SAGA, Megiddo, Itápolis, QLD, SANATS |

On racing tasks of the WGS84 competitions, the computed legs match the published ones to within 0.002%. The legs of the sphere competitions are consistently 0.0035% to 0.0042% shorter than on a 6371 km sphere. AAT leg distances refer to target points and match less precisely.

### Multiple starts

Alternative start points appear as additional `task_points` entries with `multiple_start` set. They are not part of the leg sequence. None of the sampled tasks used them. The encoding comes from the SoaringSpot importer of onglide, which skips points with `multiple_start` set when building legs.

## Daily results page

The daily results page (`/results/{class}/task-{n}-on-{date}/daily`) contains one table row per competitor. Optional columns differ between competitions: `Club` and `Handicap` are not always present.

A day without scores shows "No Results yet for this task." instead of a table.

### Precise values

Most numeric cells carry a `data-value` attribute that is more precise than the displayed text:

| column | `data-value` |
|---|---|
| Start, Finish | `YYYYMMDDhhmmss` |
| Time | seconds |
| Speed | metres per second |
| Distance | metres |
| Points | sort key, see below |

The date part of the Start and Finish `data-value` is wrong. A start on the Tábor Club class task of 2025-06-19 has the `data-value` `20250612122359`. Only the time of day is usable.

### Times

All times are competition local time. No time zone is published anywhere on the site. On the Tábor Club class task of 2025-06-19, a start shown as `12:23:59` matches the IGC fixes at 10:23 UTC (UTC+2). On an Itápolis task, a start shown as `13:38:27` falls within the flight at 16:38 UTC, and the logger header declares UTC-3.

### Points

The displayed points are the score. The Points `data-value` is `points × 10000 + tiebreaker`, where the tiebreaker orders competitors with equal points. On 1394 of 1407 sampled rows the tiebreaker was between 0 and 10000. The exceptions were 13 rows on a QLD page with a slightly negative tiebreaker.

Points are not always integers between 0 and 1000. QLD 2026 scored an elapsed-time race where the points are the gap to the leader in minutes and seconds (`-10.06` means 10 min 6 s behind). Maximum points below 1000 are common, for example 261 and 835.

### Penalties and remarks

The Points cell may contain one warning icon with a tooltip in `data-content`. Icons with the `penalty-points` class are scored penalties. All 62 sampled penalty tooltips end in `(N penalty points)`, and `N` may contain thousands separators, as in `(1,000 penalty points)`. The rest of the text is written by the scorer. Without such text, the tooltip only consists of `N penalty points`, as on two EuregioCup days. Numbers inside it are not reliable, as in `Start outside PEV interval (-60) (7 penalty points)`. Several items may be combined in one tooltip with a single total.

Icons without the `penalty-points` class carry remarks without points, such as first-offence warnings, notes about secondary loggers, or `DNF`.

### Task info

The line above the table holds the day's scoring parameters as free text, in a format that depends on the scoring software and language. Examples include English FAI scoring (`Dm = 100km, D1 = 250km, ... N: 41, n1: 39, n2: 28, Do: 421.94km, Vo: 94.78km/h`), Portuguese (`Pontuação Máxima: 261 ... Fator do DIA`), and a free-form explanation of custom scoring. FAI scoring lines end with a hash of the scoring algorithm.

## Scoring parameters

FAI Sporting Code Section 3 Annex A (2025, sections 8.3 and 8.4) and the DAeC SWO (2025, section 13) define the same daily scoring parameters and formulas. The DAeC SWO calls `D1` "DM".

| parameter | definition | derived from |
|---|---|---|
| `Dm`, `D1` | minimum distance to validate the day and for 1000 points | class constants, for example 100 km and 250 km for the Club class |
| `N` | competitors with a competition launch | rows without a non-flying status |
| `n1` | competitors with handicapped distance of at least `Dm` | Distance and Handicap |
| `n2` | finishers with handicapped speed above 2/3 of `Vo` | Speed and Handicap |
| `Do` | highest handicapped distance | Distance and Handicap |
| `Vo`, `To` | highest finisher's handicapped speed and that finisher's time | Speed, Handicap, Time |
| `Pm`, `F`, `FCR`, `Pvm`, `Pdm` | day factors and maximum points | formulas from the parameters above |

On the Tábor Club class task of 2025-06-19, these definitions applied to the page's precise values reproduce every published parameter (`N` 41, `n1` 39, `n2` 28, `Do` 421.94 km, `Vo` 94.78 km/h, `F` 1.000, `Fcr` 1.000, 455 maximum speed points) and the published score of all 41 competitors after subtracting their penalty points.

## CUP observation zone keys

The CUP specification documents only `ObsZone`, `Style`, `R1`, `A1`, `R2`, `A2`, and `A12`. Several more keys are in use:

| key | meaning |
|---|---|
| `Line=1` | Line zone. XCSoar treats `R1` as half the line width, matching SoaringSpot's "Line 15.00 km (Radius 7.50 km)". |
| `MaxAlt` | XCSoar reads it only on the start zone, as the maximum start altitude. |
| `Reduce=1` | Leg distance is reduced by the zone radius, as described above. XCSoar parses it but does not use it. |
| `Move`, `SpeedStyle` | Meaning unknown. |

## Sources

- [FAI Sporting Code Section 3 Annex A, 2025 edition](https://www.fai.org/sites/default/files/sc3a_2025.pdf)
- [DAeC SWO 2025](https://www.daec.de/media/files/Dateien/Sportarten/Segelflug/Downloads/SWO2025_20250228.pdf)
- [CUP file format](https://github.com/naviter/seeyou_file_formats/blob/main/CUP_file_format.md)
- [XCSoar `TaskFileSeeYou.cpp`](https://github.com/XCSoar/XCSoar/blob/master/src/Task/TaskFileSeeYou.cpp)
- [aerofiles `seeyou/reader.py`](https://github.com/Turbo87/aerofiles/blob/master/aerofiles/seeyou/reader.py)
- [onglide `lib/scoring/shared/tasks.ts`](https://github.com/ifly7charlie/onglide/blob/main/lib/scoring/shared/tasks.ts)
