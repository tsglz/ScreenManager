import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import { api, Report, ReportListItem } from '../utils/api'
import './HistoryReports.css'

// —————————— 常量 ——————————
const TEMPLATES = [
  { key: 'standard', label: '标准' },
  { key: 'tech', label: '技术' },
  { key: 'project', label: '项目' },
  { key: 'concise', label: '简洁' },
  { key: 'pomodoro', label: '番茄钟' },
] as const
type TemplateKey = (typeof TEMPLATES)[number]['key']

type PresetKey = 'day' | '7' | '30' | 'month' | 'lastmonth' | 'custom'

const PERIOD_FILTERS = [
  { key: '', label: '全部周期' },
  { key: 'daily', label: '日报' },
  { key: 'weekly', label: '周报' },
  { key: 'monthly', label: '月报' },
]
const TYPE_FILTERS = [
  { key: '', label: '全部模板' },
  ...TEMPLATES.map(t => ({ key: t.key, label: `${t.label}` })),
]

const PAGE_SIZE = 20

// —————————— 工具函数 ——————————
function pad(n: number) { return n.toString().padStart(2, '0') }
function fmtDate(d: Date) { return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` }
function todayStr() { return fmtDate(new Date()) }

function relTime(at: string): string {
  // at: YYYY-MM-DD HH:MM:SS
  const t = new Date(at.replace(' ', 'T')).getTime()
  if (isNaN(t)) return at
  const diff = Date.now() - t
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return '刚刚'
  if (mins < 60) return `${mins} 分钟前`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs} 小时前`
  const days = Math.floor(hrs / 24)
  if (days < 30) return `${days} 天前`
  return at.slice(0, 10)
}

function periodicityLabel(p: string) {
  return p === 'daily' ? '日报' : p === 'weekly' ? '周报' : '月报'
}
function periodicityClass(p: string) {
  return p === 'daily' ? 'period-daily' : p === 'weekly' ? 'period-weekly' : 'period-monthly'
}
function templateLabel(k: string) {
  return TEMPLATES.find(t => t.key === k)?.label ?? k
}

// —————————— Markdown 轻量渲染器 ——————————
type MdNode =
  | { kind: 'h1' | 'h2' | 'h3'; text: string }
  | { kind: 'p'; text: string }
  | { kind: 'ul'; items: string[] }
  | { kind: 'ol'; items: string[] }
  | { kind: 'pre'; code: string; lang?: string }
  | { kind: 'blockquote'; text: string }
  | { kind: 'hr' }
  | { kind: 'table'; header: string[]; rows: string[][] }

function renderInline(text: string, keyPrefix: string): React.ReactNode[] {
  // 顺序：代码 → 粗体 → 斜体
  const out: React.ReactNode[] = []
  let i = 0
  let k = 0
  while (i < text.length) {
    let matched = false
    // code: `xxx`
    if (text[i] === '`') {
      const end = text.indexOf('`', i + 1)
      if (end > i) {
        const code = text.slice(i + 1, end)
        out.push(<code key={`${keyPrefix}-c${k++}`}>{code}</code>)
        i = end + 1
        matched = true
      }
    }
    if (!matched && text.startsWith('**', i)) {
      const end = text.indexOf('**', i + 2)
      if (end > i + 1) {
        const inner = text.slice(i + 2, end)
        out.push(<strong key={`${keyPrefix}-b${k++}`}>{renderInline(inner, `${keyPrefix}-b${k}`)}</strong>)
        i = end + 2
        matched = true
      }
    }
    if (!matched && text[i] === '*' && i + 1 < text.length && text[i + 1] !== '*') {
      const end = text.indexOf('*', i + 1)
      if (end > i) {
        const inner = text.slice(i + 1, end)
        out.push(<em key={`${keyPrefix}-i${k++}`}>{renderInline(inner, `${keyPrefix}-i${k}`)}</em>)
        i = end + 1
        matched = true
      }
    }
    if (!matched) {
      // find next special char start
      let j = i + 1
      while (j < text.length && !['`', '*'].includes(text[j])) j++
      out.push(<React.Fragment key={`${keyPrefix}-t${k++}`}>{text.slice(i, j)}</React.Fragment>)
      i = j
    }
  }
  return out
}

function parseMd(md: string): MdNode[] {
  const lines = md.split(/\r?\n/)
  const out: MdNode[] = []
  let i = 0
  while (i < lines.length) {
    const line = lines[i]
    if (line.trim() === '') { i++; continue }
    if (line.startsWith('```')) {
      const lang = line.slice(3).trim()
      const codeLines: string[] = []
      i++
      while (i < lines.length && !lines[i].startsWith('```')) {
        codeLines.push(lines[i])
        i++
      }
      i++ // skip closing ```
      out.push({ kind: 'pre', code: codeLines.join('\n'), lang: lang || undefined })
      continue
    }
    if (/^#{1,3}\s+/.test(line)) {
      const level = (line.match(/^#+/) || [''])[0].length as 1 | 2 | 3
      const text = line.replace(/^#{1,3}\s+/, '')
      out.push({ kind: (`h${level}` as 'h1' | 'h2' | 'h3'), text })
      i++
      continue
    }
    if (/^\s*[-*]\s+/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ''))
        i++
      }
      out.push({ kind: 'ul', items })
      continue
    }
    if (/^\s*\d+\.\s+/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ''))
        i++
      }
      out.push({ kind: 'ol', items })
      continue
    }
    if (line.startsWith('> ')) {
      const parts: string[] = []
      while (i < lines.length && lines[i].startsWith('> ')) {
        parts.push(lines[i].slice(2))
        i++
      }
      out.push({ kind: 'blockquote', text: parts.join(' ') })
      continue
    }
    if (/^---+$/.test(line.trim())) {
      out.push({ kind: 'hr' })
      i++
      continue
    }
    if (line.includes('|') && lines[i + 1] && /^[-|\s:]+$/.test(lines[i + 1]?.trim() || '')) {
      const header = line.split('|').map(s => s.trim()).filter(s => s.length)
      i += 2
      const rows: string[][] = []
      while (i < lines.length && lines[i].includes('|')) {
        const row = lines[i].split('|').map(s => s.trim()).filter(s => s.length)
        if (row.length === 0) break
        rows.push(row)
        i++
      }
      out.push({ kind: 'table', header, rows })
      continue
    }
    // paragraph
    const parts: string[] = [line]
    i++
    while (i < lines.length && lines[i].trim() !== '' && !/^(#{1,3}|```|>|---|\s*[-*]\s|\s*\d+\.\s)/.test(lines[i])) {
      parts.push(lines[i])
      i++
    }
    out.push({ kind: 'p', text: parts.join(' ') })
  }
  return out
}

function renderMarkdown(md: string): JSX.Element[] {
  const nodes = parseMd(md)
  return nodes.map((n, idx) => {
    const k = `md-${idx}`
    switch (n.kind) {
      case 'h1': return <h1 key={k}>{renderInline(n.text, k)}</h1>
      case 'h2': return <h2 key={k}>{renderInline(n.text, k)}</h2>
      case 'h3': return <h3 key={k}>{renderInline(n.text, k)}</h3>
      case 'p':  return <p key={k}>{renderInline(n.text, k)}</p>
      case 'ul': return <ul key={k}>{n.items.map((it, j) => <li key={j}>{renderInline(it, `${k}-li${j}`)}</li>)}</ul>
      case 'ol': return <ol key={k}>{n.items.map((it, j) => <li key={j}>{renderInline(it, `${k}-li${j}`)}</li>)}</ol>
      case 'pre': return <pre key={k}><code className={n.lang ? `lang-${n.lang}` : undefined}>{n.code}</code></pre>
      case 'blockquote': return <blockquote key={k}>{renderInline(n.text, k)}</blockquote>
      case 'hr': return <hr key={k} />
      case 'table': return (
        <table key={k}>
          <thead><tr>{n.header.map((h, j) => <th key={j}>{renderInline(h, `${k}-th${j}`)}</th>)}</tr></thead>
          <tbody>{n.rows.map((row, r) => <tr key={r}>{row.map((c, j) => <td key={j}>{renderInline(c, `${k}-r${r}c${j}`)}</td>)}</tr>)}</tbody>
        </table>
      )
    }
  })
}

// —————————— 主组件 ——————————
export default function HistoryReports() {
  // === 新建面板状态 ===
  const [tpl, setTpl] = useState<TemplateKey>('standard')
  const [preset, setPreset] = useState<PresetKey>('day')
  const today = todayStr()
  const [customStart, setCustomStart] = useState(today)
  const [customEnd, setCustomEnd] = useState(today)
  const [generating, setGenerating] = useState(false)

  // === 筛选/搜索/分页 ===
  const [keyword, setKeyword] = useState('')
  const debouncedKw = useDebounce(keyword, 300)
  const [filterType, setFilterType] = useState('')
  const [filterPeriod, setFilterPeriod] = useState('')
  const [page, setPage] = useState(1)
  const [listLoading, setListLoading] = useState(false)
  const [result, setResult] = useState<{ items: ReportListItem[]; total: number }>({ items: [], total: 0 })

  // === 预览 ===
  const [preview, setPreview] = useState<Report | null>(null)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [actionStatus, setActionStatus] = useState<Record<string, string>>({})

  // === 范围计算 ===
  const range = useMemo(() => {
    const now = new Date()
    switch (preset) {
      case 'day': {
        return { start: today, end: today }
      }
      case '7': {
        const s = new Date(now.valueOf() - 6 * 86400000)
        return { start: fmtDate(s), end: today }
      }
      case '30': {
        const s = new Date(now.valueOf() - 29 * 86400000)
        return { start: fmtDate(s), end: today }
      }
      case 'month': {
        const s = new Date(now.getFullYear(), now.getMonth(), 1)
        return { start: fmtDate(s), end: today }
      }
      case 'lastmonth': {
        const firstThis = new Date(now.getFullYear(), now.getMonth(), 1)
        const last = new Date(firstThis.valueOf() - 86400000)
        const s = new Date(last.getFullYear(), last.getMonth(), 1)
        return { start: fmtDate(s), end: fmtDate(last) }
      }
      case 'custom':
      default:
        return { start: customStart, end: customEnd }
    }
  }, [preset, customStart, customEnd, today])

  // === 列表刷新 ===
  const refreshList = useCallback(async () => {
    setListLoading(true)
    try {
      const r = await api.listReports(debouncedKw, filterType, filterPeriod, page, PAGE_SIZE)
      setResult(r)
    } catch (e) {
      console.error(e)
      setResult({ items: [], total: 0 })
    } finally {
      setListLoading(false)
    }
  }, [debouncedKw, filterType, filterPeriod, page])

  useEffect(() => {
    setPage(1)
  }, [debouncedKw, filterType, filterPeriod])

  useEffect(() => {
    refreshList()
  }, [refreshList, page])

  // === 预览打开（两种方式：内容已知直接开 / 按 id 查） ===
  const openPreview = useCallback(async (id: number, _contentHint?: string) => {
    setPreviewLoading(true)
    try {
      const r = await api.getReport(id)
      if (r) {
        setPreview(r)
      } else {
        alert('报告不存在或已删除')
      }
    } catch (e) {
      alert('加载报告失败')
    } finally {
      setPreviewLoading(false)
    }
  }, [])

  // === 生成报告 ===
  const handleGenerate = async () => {
    if (generating) return
    setGenerating(true)
    try {
      // —— Step 1：快速健康检查（~2s），尽早发现 Ollama 未启动/模型未 pull ——
      try {
        await api.probeOllamaReady()
      } catch (probeErr) {
        alert(`Ollama 未就绪：${String(probeErr)}`)
        setGenerating(false)
        return
      }
      // —— Step 2：健康检查通过后再真正调用 AI 生成 ——
      const [_id, _content] = await api.createAndSaveReport(tpl, range.start, range.end)
      setPage(1)
      await refreshList()
      // 重新拉刚生成的报告并预览
      if (_id > 0) {
        await openPreview(_id)
      }
    } catch (err: any) {
      const msg = String(err?.message || err)
      alert(`生成失败：${msg}`)
    } finally {
      setGenerating(false)
    }
  }

  // === 6 个动作 ===
  const setTempStatus = (key: string, text: string, ms = 2200) => {
    setActionStatus(prev => ({ ...prev, [key]: text }))
    window.setTimeout(() => {
      setActionStatus(prev => {
        const copy = { ...prev }
        delete copy[key]
        return copy
      })
    }, ms)
  }

  const copyContent = async (content: string, tag: string) => {
    try {
      await navigator.clipboard.writeText(content)
      setTempStatus(`copy-${tag}`, '已复制 ✔')
    } catch (e) {
      alert('复制失败，请手动复制')
    }
  }

  const exportMd = async (id: number, title: string) => {
    const safeName = title.replace(/[\\/:*?"<>|]/g, '-') + '.md'
    try {
      const path = await saveDialog({
        defaultPath: safeName,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      })
      if (!path) return
      const ok = await api.exportReportToFile(id, path as string)
      if (ok) {
        setTempStatus(`export-${id}`, '已导出 ✔')
      } else {
        alert('导出失败，路径无权限或报告不存在')
      }
    } catch (e) {
      alert('导出取消或失败')
    }
  }

  const doDelete = async (id: number) => {
    if (!confirm('确定删除此报告？此操作不可恢复。')) return
    const ok = await api.deleteReport(id)
    if (ok) {
      setResult(prev => ({ ...prev, items: prev.items.filter(x => x.id !== id), total: Math.max(0, prev.total - 1) }))
      if (preview?.id === id) setPreview(null)
    } else {
      alert('删除失败')
    }
  }

  const doRegenerate = async (item: ReportListItem) => {
    if (!confirm(`用相同参数重新生成「${item.title}」？会覆盖旧内容。`)) return
    setGenerating(true)
    try {
      await api.createAndSaveReport(item.report_type, item.start_date, item.end_date)
      await refreshList()
      if (preview?.id === item.id) {
        // 刷新预览内容
        const fresh = await api.getReport(item.id)
        if (fresh) setPreview(fresh)
      }
      setTempStatus(`regen-${item.id}`, '已重新生成 ✔')
    } catch (err: any) {
      alert(`重新生成失败：${String(err?.message || err)}\n旧内容已保留。`)
    } finally {
      setGenerating(false)
    }
  }

  // === 分页计算 ===
  const totalPages = Math.max(1, Math.ceil(result.total / PAGE_SIZE))
  const pageNums = useMemo(() => {
    const arr: number[] = []
    const first = Math.max(1, page - 2)
    const last = Math.min(totalPages, first + 4)
    for (let i = first; i <= last; i++) arr.push(i)
    return arr
  }, [page, totalPages])

  return (
    <div className="hr-page">
      {/* ===== 新建报告卡片 ===== */}
      <div className="hr-new-card">
        <h3 className="hr-new-title">生成新报告</h3>
        <div className="hr-row">
          <span className="hr-label">模板</span>
          <div className="hr-templates">
            {TEMPLATES.map(t => (
              <button
                key={t.key}
                className={'hr-tpl-btn' + (tpl === t.key ? ' active' : '')}
                onClick={() => setTpl(t.key)}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>
        <div className="hr-row">
          <span className="hr-label">范围</span>
          <div className="hr-preset-group">
            {(['day','7','30','month','lastmonth','custom'] as PresetKey[]).map(k => {
              const label = k === 'day' ? '今日' : k === '7' ? '近7天' : k === '30' ? '近30天' : k === 'month' ? '本月' : k === 'lastmonth' ? '上月' : '自定义'
              return (
                <button key={k}
                  className={'hr-preset-btn' + (preset === k ? ' active' : '')}
                  onClick={() => setPreset(k)}
                >{label}</button>
              )
            })}
          </div>
          {preset === 'custom' && (
            <div className="hr-custom-range">
              <label>起
                <input type="date" max={today} value={customStart}
                  onChange={e => setCustomStart(e.target.value)} />
              </label>
              <label>至
                <input type="date" max={today} value={customEnd}
                  onChange={e => setCustomEnd(e.target.value)} />
              </label>
            </div>
          )}
          <button
            className="hr-generate-btn"
            disabled={generating || range.start > range.end}
            onClick={handleGenerate}
          >
            {generating ? '生成中…（可能需要几十秒）' : '生成报告'}
          </button>
        </div>
      </div>

      {/* ===== 筛选条 ===== */}
      <div className="hr-filter-bar">
        <input
          className="hr-search"
          placeholder="搜索报告标题或内容…"
          value={keyword}
          onChange={e => setKeyword(e.target.value)}
        />
        <select className="hr-select" value={filterType}
          onChange={e => setFilterType(e.target.value)}>
          {TYPE_FILTERS.map(f => <option key={f.key} value={f.key}>{f.label}</option>)}
        </select>
        <select className="hr-select" value={filterPeriod}
          onChange={e => setFilterPeriod(e.target.value)}>
          {PERIOD_FILTERS.map(f => <option key={f.key} value={f.key}>{f.label}</option>)}
        </select>
        <span className="hr-count-hint">共 {result.total} 条</span>
      </div>

      {/* ===== 列表 ===== */}
      {listLoading ? <div style={{ padding: '30px 0', textAlign: 'center', color: 'var(--text-tertiary)' }}>加载中…</div> :
       result.items.length === 0 ? <div className="hr-empty">暂无报告，试试从上方生成第一份~</div> : (
        <div className="hr-list">
          {result.items.map(item => (
            <div className="hr-report-card" key={item.id}>
              <div className="hr-card-title">{item.title}</div>
              <div className="hr-card-range">{item.start_date} ~ {item.end_date} · 更新于 {relTime(item.updated_at)}</div>
              <div className="hr-card-tags">
                <span className={'hr-tag ' + periodicityClass(item.periodicity)}>
                  {periodicityLabel(item.periodicity)}
                </span>
                <span className="hr-tag">{templateLabel(item.report_type)}模板</span>
                {actionStatus[`regen-${item.id}`] && <span className="hr-tag" style={{ color: '#16a34a', borderColor: '#86efac' }}>
                  {actionStatus[`regen-${item.id}`]}
                </span>}
              </div>
              <div className="hr-actions">
                <button className="hr-btn" onClick={() => openPreview(item.id)}>查看</button>
                <button className="hr-btn ghost" onClick={() => {
                  // 先拿内容再复制
                  api.getReport(item.id).then(r => { if (r) copyContent(r.content_md, `card-${item.id}`) })
                }}>{actionStatus[`copy-card-${item.id}`] || '复制'}</button>
                <button className="hr-btn ghost" onClick={() => exportMd(item.id, item.title)}>
                  {actionStatus[`export-${item.id}`] || '导出.md'}
                </button>
                <button className="hr-btn ghost" onClick={() => doRegenerate(item)} disabled={generating}>
                  重新生成
                </button>
                <button className="hr-btn danger" onClick={() => doDelete(item.id)}>删除</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ===== 分页 ===== */}
      {result.items.length > 0 && (
        <div className="hr-pagination">
          <button className="hr-page-btn" disabled={page <= 1 || listLoading} onClick={() => setPage(p => Math.max(1, p - 1))}>上一页</button>
          {pageNums.map(n => (
            <button key={n}
              className={'hr-page-btn' + (n === page ? ' active' : '')}
              onClick={() => setPage(n)}>{n}</button>
          ))}
          <button className="hr-page-btn" disabled={page >= totalPages || listLoading} onClick={() => setPage(p => Math.min(totalPages, p + 1))}>下一页</button>
          <span className="hr-page-info">第 {page} / {totalPages} 页</span>
        </div>
      )}

      {/* ===== 预览面板 ===== */}
      {preview && (
        <div className="hr-preview-mask" onClick={(e) => {
          if ((e.target as HTMLElement).classList.contains('hr-preview-mask')) setPreview(null)
        }}>
          <div className="hr-preview-drawer" onClick={e => e.stopPropagation()}>
            <div className="hr-preview-header" style={{ position: 'relative' }}>
              <button className="hr-preview-close" onClick={() => setPreview(null)} title="关闭">✕</button>
              <h4 className="hr-preview-title">{preview.title}</h4>
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                {preview.start_date} ~ {preview.end_date} · 类型：{templateLabel(preview.report_type)} · 周期：{periodicityLabel(preview.periodicity)}
                {preview.updated_at !== preview.created_at && ` · 更新于 ${preview.updated_at}`}
              </div>
              <div className="hr-preview-tools">
                <button className="hr-btn ghost" onClick={() => copyContent(preview.content_md, `preview-${preview.id}`)}>
                  {actionStatus[`copy-preview-${preview.id}`] || '复制全文'}
                </button>
                <button className="hr-btn ghost" onClick={() => exportMd(preview.id, preview.title)}>
                  {actionStatus[`export-${preview.id}`] || '导出 .md'}
                </button>
                <button className="hr-btn ghost" onClick={() => doRegenerate({ ...preview } as ReportListItem)} disabled={generating}>重新生成</button>
                <button className="hr-btn danger" onClick={() => doDelete(preview.id)}>删除</button>
              </div>
            </div>
            <div className="hr-preview-body">
              {previewLoading && <div style={{ padding: 20, color: 'var(--text-tertiary)' }}>加载中…</div>}
              {!previewLoading && <div className="md-preview">{renderMarkdown(preview.content_md)}</div>}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

// —————————— 通用 hook：useDebounce ——————————
function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value)
  const timer = useRef<number | null>(null)
  useEffect(() => {
    if (timer.current) window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => setDebounced(value), delay)
    return () => { if (timer.current) window.clearTimeout(timer.current) }
  }, [value, delay])
  return debounced
}
