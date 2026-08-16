import React, { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, HourlyHeatmapEntry } from '../utils/api'
import './TimeHeatmap.css'

type Dimension = 'day' | 'week' | 'month'

interface GridData {
  rowLabels: string[]
  colCount: number
  colLabels: string[]
  cells: number[][]
  rowDrillDown: { start: string; end: string }[]
  cellDrillDown: { start: string; end: string }[][]
  maxDuration: number
}

function pad(n: number) {
  return n.toString().padStart(2, '0')
}

function fmtDate(d: Date) {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

function parseDate(s: string) {
  const [y, m, d] = s.split('-').map(Number)
  return new Date(y, m - 1, d)
}

function daysBetween(a: string, b: string) {
  const d1 = parseDate(a).getTime()
  const d2 = parseDate(b).getTime()
  return Math.round((d2 - d1) / 86400000) + 1
}

function weekdayCn(d: Date) {
  return ['日', '一', '二', '三', '四', '五', '六'][d.getDay()]
}

function getIsoWeek(d: Date): { year: number; week: number } {
  const date = new Date(d.valueOf())
  date.setHours(0, 0, 0, 0)
  date.setDate(date.getDate() + 3 - ((date.getDay() + 6) % 7))
  const week1 = new Date(date.getFullYear(), 0, 4)
  const week =
    1 +
    Math.round(
      ((date.valueOf() - week1.valueOf()) / 86400000 - 3 + ((week1.getDay() + 6) % 7)) / 7,
    )
  return { year: date.getFullYear(), week }
}

function isoWeekKey(d: Date) {
  const { year, week } = getIsoWeek(d)
  return `${year}-W${pad(week)}`
}

function mondayOfIsoWeek(year: number, week: number): Date {
  const jan4 = new Date(year, 0, 4)
  const jan4MondayOffset = ((jan4.getDay() + 6) % 7)
  jan4.setDate(jan4.getDate() - jan4MondayOffset)
  return new Date(jan4.getFullYear(), jan4.getMonth(), jan4.getDate() + (week - 1) * 7)
}

function getHeatLevel(duration: number, max: number): number {
  if (max === 0 || duration === 0) return 0
  const ratio = duration / max
  if (ratio <= 0.15) return 1
  if (ratio <= 0.35) return 2
  if (ratio <= 0.55) return 3
  if (ratio <= 0.75) return 4
  return 5
}

function fmtDuration(sec: number) {
  if (sec < 60) return `${sec}s`
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (h === 0) return `${m}m`
  if (m === 0) return `${h}h`
  return `${h}h ${m}m`
}

function aggregateByDimension(
  data: HourlyHeatmapEntry[],
  dimension: Dimension,
  startDate: string,
  endDate: string,
): GridData {
  const start = parseDate(startDate)
  const totalDays = daysBetween(startDate, endDate)

  if (dimension === 'day') {
    const rowCount = totalDays
    const colCount = 24
    const cells: number[][] = Array.from({ length: rowCount }, () => Array(colCount).fill(0))
    const rowLabels: string[] = []
    const rowDrillDown: { start: string; end: string }[] = []
    const cellDrillDown: { start: string; end: string }[][] = []

    const dateToRow = new Map<string, number>()
    for (let i = 0; i < rowCount; i++) {
      const d = new Date(start.valueOf() + i * 86400000)
      const k = fmtDate(d)
      dateToRow.set(k, i)
      rowLabels.push(`${pad(d.getMonth() + 1)}/${pad(d.getDate())} 周${weekdayCn(d)}`)
      rowDrillDown.push({ start: k, end: k })
    }

    for (const e of data) {
      const r = dateToRow.get(e.date)
      if (r === undefined) continue
      const h = Math.max(0, Math.min(23, e.hour))
      cells[r][h] += e.duration_seconds || 0
    }

    for (let r = 0; r < rowCount; r++) {
      const d = new Date(start.valueOf() + r * 86400000)
      const k = fmtDate(d)
      const rowCells: { start: string; end: string }[] = []
      for (let c = 0; c < colCount; c++) rowCells.push({ start: k, end: k })
      cellDrillDown.push(rowCells)
    }

    const colLabels: string[] = Array.from({ length: colCount }, (_, i) => (i % 3 === 0 ? `${i}:00` : ''))

    let maxDuration = 0
    for (let r = 0; r < rowCount; r++) for (let c = 0; c < colCount; c++) if (cells[r][c] > maxDuration) maxDuration = cells[r][c]

    return { rowLabels, colCount, colLabels, cells, rowDrillDown, cellDrillDown, maxDuration }
  }

  if (dimension === 'week') {
    const weekKeys: string[] = []
    const weekKeyInfo = new Map<string, { year: number; week: number; monday: Date; sunday: Date }>()
    for (let i = 0; i < totalDays; i++) {
      const d = new Date(start.valueOf() + i * 86400000)
      const { year, week } = getIsoWeek(d)
      const key = isoWeekKey(d)
      if (weekKeyInfo.has(key)) continue
      const monday = mondayOfIsoWeek(year, week)
      const sunday = new Date(monday.valueOf() + 6 * 86400000)
      weekKeyInfo.set(key, { year, week, monday, sunday })
      weekKeys.push(key)
    }
    const rowCount = weekKeys.length
    const colCount = 24
    const cells: number[][] = Array.from({ length: rowCount }, () => Array(colCount).fill(0))
    const rowLabels: string[] = weekKeys.map(k => `第${weekKeyInfo.get(k)!.week}周`)
    const rowDrillDown: { start: string; end: string }[] = weekKeys.map(k => {
      const { monday, sunday } = weekKeyInfo.get(k)!
      return { start: fmtDate(monday), end: fmtDate(sunday) }
    })
    const cellDrillDown: { start: string; end: string }[][] = weekKeys.map(k => {
      const { monday, sunday } = weekKeyInfo.get(k)!
      const s = fmtDate(monday), e = fmtDate(sunday)
      return Array.from({ length: colCount }, () => ({ start: s, end: e }))
    })

    const weekKeyToRow = new Map(weekKeys.map((k, i) => [k, i]))
    for (const e of data) {
      const d = parseDate(e.date)
      const k = isoWeekKey(d)
      const r = weekKeyToRow.get(k)
      if (r === undefined) continue
      const h = Math.max(0, Math.min(23, e.hour))
      cells[r][h] += e.duration_seconds || 0
    }

    const colLabels: string[] = Array.from({ length: colCount }, (_, i) => (i % 3 === 0 ? `${i}:00` : ''))

    let maxDuration = 0
    for (let r = 0; r < rowCount; r++) for (let c = 0; c < colCount; c++) if (cells[r][c] > maxDuration) maxDuration = cells[r][c]

    return { rowLabels, colCount, colLabels, cells, rowDrillDown, cellDrillDown, maxDuration }
  }

  const monthKeys: string[] = []
  const monthInfo = new Map<string, { year: number; month: number; daysInMonth: number }>()
  for (let i = 0; i < totalDays; i++) {
    const d = new Date(start.valueOf() + i * 86400000)
    const y = d.getFullYear()
    const m = d.getMonth() + 1
    const key = `${y}-${pad(m)}`
    if (!monthInfo.has(key)) {
      const daysInMonth = new Date(y, m, 0).getDate()
      monthInfo.set(key, { year: y, month: m, daysInMonth })
      monthKeys.push(key)
    }
  }
  const rowCount = monthKeys.length
  const colCount = 31
  const cells: number[][] = Array.from({ length: rowCount }, () => Array(colCount).fill(0))
  const rowLabels: string[] = monthKeys.map(k => k)
  const rowDrillDown: { start: string; end: string }[] = monthKeys.map(k => {
    const { year, month, daysInMonth } = monthInfo.get(k)!
    return {
      start: `${year}-${pad(month)}-01`,
      end: `${year}-${pad(month)}-${pad(daysInMonth)}`,
    }
  })
  const cellDrillDown: { start: string; end: string }[][] = monthKeys.map(k => {
    const { year, month, daysInMonth } = monthInfo.get(k)!
    const s = `${year}-${pad(month)}-01`
    const e = `${year}-${pad(month)}-${pad(daysInMonth)}`
    return Array.from({ length: colCount }, (_, d) => {
      const dayNum = d + 1
      if (dayNum > daysInMonth) return { start: s, end: e }
      return {
        start: `${year}-${pad(month)}-${pad(dayNum)}`,
        end: `${year}-${pad(month)}-${pad(Math.min(dayNum, daysInMonth))}`,
      }
    })
  })

  const monthKeyToRow = new Map(monthKeys.map((k, i) => [k, i]))
  for (const e of data) {
    const key = e.date.slice(0, 7)
    const r = monthKeyToRow.get(key)
    if (r === undefined) continue
    const dayNum = parseInt(e.date.slice(8, 10), 10)
    const col = Math.max(0, Math.min(30, dayNum - 1))
    cells[r][col] += e.duration_seconds || 0
  }

  const colLabels: string[] = Array.from({ length: colCount }, (_, i) => {
    const dayNum = i + 1
    return dayNum === 1 || dayNum % 5 === 0 ? String(dayNum) : ''
  })

  let maxDuration = 0
  for (let r = 0; r < rowCount; r++) for (let c = 0; c < colCount; c++) if (cells[r][c] > maxDuration) maxDuration = cells[r][c]

  return { rowLabels, colCount, colLabels, cells, rowDrillDown, cellDrillDown, maxDuration }
}

type PresetKey = '7' | '30' | 'month' | 'lastmonth' | 'custom'

export default function TimeHeatmap() {
  const navigate = useNavigate()
  const today = useMemo(() => {
    const d = new Date()
    return fmtDate(d)
  }, [])

  const [preset, setPreset] = useState<PresetKey>('7')
  const [customStart, setCustomStart] = useState<string>(today)
  const [customEnd, setCustomEnd] = useState<string>(today)
  const [loading, setLoading] = useState(false)
  const [data, setData] = useState<HourlyHeatmapEntry[]>([])
  const [error, setError] = useState<string | null>(null)

  const range = useMemo(() => {
    if (preset === 'custom') {
      return { start: customStart, end: customEnd, preset }
    }
    const now = new Date()
    if (preset === '7') {
      const s = new Date(now.valueOf() - 6 * 86400000)
      return { start: fmtDate(s), end: today, preset }
    }
    if (preset === '30') {
      const s = new Date(now.valueOf() - 29 * 86400000)
      return { start: fmtDate(s), end: today, preset }
    }
    if (preset === 'month') {
      const s = new Date(now.getFullYear(), now.getMonth(), 1)
      return { start: fmtDate(s), end: today, preset }
    }
    const firstThis = new Date(now.getFullYear(), now.getMonth(), 1)
    const last = new Date(firstThis.valueOf() - 86400000)
    const s = new Date(last.getFullYear(), last.getMonth(), 1)
    return { start: fmtDate(s), end: fmtDate(last), preset }
  }, [preset, customStart, customEnd, today])

  const dimension: Dimension = useMemo(() => {
    const n = daysBetween(range.start, range.end)
    if (n <= 7) return 'day'
    if (n <= 31) return 'week'
    return 'month'
  }, [range.start, range.end])

  const rangeValid = useMemo(() => {
    const n = daysBetween(range.start, range.end)
    return n > 0 && n <= 365
  }, [range.start, range.end])

  useEffect(() => {
    let cancelled = false
    if (!rangeValid) {
      setData([])
      setError('范围无效或超过 365 天')
      return
    }
    setError(null)
    setLoading(true)
    api
      .getHourlyHeatmapForRange(range.start, range.end)
      .then(res => {
        if (cancelled) return
        setData(res)
      })
      .catch(e => {
        if (cancelled) return
        setError(String(e?.message || e))
        setData([])
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [range.start, range.end, rangeValid])

  const grid = useMemo(
    () => aggregateByDimension(data, dimension, range.start, range.end),
    [data, dimension, range.start, range.end],
  )

  const dimensionLabel = dimension === 'day' ? '日视图' : dimension === 'week' ? '周视图' : '月视图'
  const totalSeconds = useMemo(() => {
    let s = 0
    for (const r of grid.cells) for (const c of r) s += c
    return s
  }, [grid])

  const hoverText = (rowIdx: number, colIdx: number): string => {
    const dur = grid.cells[rowIdx][colIdx]
    const rowLabel = grid.rowLabels[rowIdx]
    const colLabel =
      dimension === 'month' ? `${colIdx + 1}日` : `${colIdx}:00`
    if (dur === 0) return `${rowLabel} ${colLabel} — 无活动`
    return `${rowLabel} ${colLabel} — ${fmtDuration(dur)}`
  }

  const onCellClick = (rowIdx: number, colIdx: number) => {
    const { start, end } = grid.cellDrillDown[rowIdx][colIdx]
    navigate(`/work-timeline?start=${start}&end=${end}`)
  }

  const presetBtn = (k: PresetKey, label: string) => (
    <button
      key={k}
      className={preset === k ? 'preset-btn preset-active' : 'preset-btn'}
      onClick={() => setPreset(k)}
    >
      {label}
    </button>
  )

  return (
    <div className="timeheatmap-page">
      <div className="th-header">
        <h2 className="th-title">时段热力图</h2>
        <div className="th-summary">
          {loading && <span className="th-hint">加载中…</span>}
          {!loading && (
            <span className="th-hint">
              {range.start} ~ {range.end} · {dimensionLabel} · 总活动时长 {fmtDuration(totalSeconds)}
            </span>
          )}
        </div>
      </div>

      <div className="th-range">
        <div className="preset-group">
          {presetBtn('7', '近 7 天')}
          {presetBtn('30', '近 30 天')}
          {presetBtn('month', '本月')}
          {presetBtn('lastmonth', '上月')}
          {presetBtn('custom', '自定义')}
        </div>
        {preset === 'custom' && (
          <div className="custom-range">
            <label>
              起
              <input
                type="date"
                max={today}
                value={customStart}
                onChange={e => setCustomStart(e.target.value)}
              />
            </label>
            <label>
              至
              <input
                type="date"
                max={today}
                value={customEnd}
                onChange={e => setCustomEnd(e.target.value)}
              />
            </label>
          </div>
        )}
      </div>

      {error && <div className="th-error">{error}</div>}

      {!loading && !error && grid.rowLabels.length === 0 ? (
        <div className="empty-state">暂无数据</div>
      ) : (
        <div className="heatmap-container">
          <div className="heatmap-scroll">
            <div
              className="heatmap-grid"
              style={{
                gridTemplateColumns: `110px repeat(${grid.colCount}, var(--cell-size))`,
              }}
            >
              <div className="hm-corner"></div>
              {grid.colLabels.map((lbl, i) => (
                <div key={i} className="hm-hour-label">
                  {lbl}
                </div>
              ))}
              {grid.rowLabels.map((rLabel, rIdx) => (
                <React.Fragment key={rIdx}>
                  <div className="hm-date-label">{rLabel}</div>
                  {grid.cells[rIdx].map((dur, cIdx) => {
                    const level = getHeatLevel(dur, grid.maxDuration)
                    return (
                      <div
                        key={cIdx}
                        className={`heat-cell level-${level}`}
                        title={hoverText(rIdx, cIdx)}
                        onClick={() => onCellClick(rIdx, cIdx)}
                      />
                    )
                  })}
                </React.Fragment>
              ))}
            </div>
          </div>

          <div className="heatmap-legend">
            <span className="legend-label">少</span>
            <span className="legend-cell level-0"></span>
            <span className="legend-cell level-1"></span>
            <span className="legend-cell level-2"></span>
            <span className="legend-cell level-3"></span>
            <span className="legend-cell level-4"></span>
            <span className="legend-cell level-5"></span>
            <span className="legend-label">多</span>
          </div>
        </div>
      )}
    </div>
  )
}
