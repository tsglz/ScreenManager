import { useEffect, useRef, useState } from 'react'
import { api, type OllamaModelInfo } from '../utils/api'
import './GenerateReport.css'

interface ReportType {
  id: string
  name: string
  icon: string
  desc: string
}

const REPORT_TYPES: ReportType[] = [
  {
    id: 'pomodoro',
    name: '番茄钟聚类',
    icon: '🍅',
    desc: '按工作类型聚类，非时间线报告',
  },
  {
    id: 'concise',
    name: '简洁日报',
    icon: '✨',
    desc: '只列关键工作，适合快速汇报',
  },
  {
    id: 'tech',
    name: '技术日报',
    icon: '💻',
    desc: '侧重代码开发和技术问题',
  },
  {
    id: 'project',
    name: '项目日报',
    icon: '📁',
    desc: '按项目维度组织工作内容',
  },
  {
    id: 'standard',
    name: '标准日报',
    icon: '📊',
    desc: '按类别归纳今日已完成工作',
  },
]

const DEFAULT_MODEL_NAME = 'qwen3:4b-fp16'

function formatDateForInput(date: Date): string {
  return date.toISOString().split('T')[0]
}

function shortModelName(name: string): string {
  if (!name) return name
  const colonIdx = name.indexOf(':')
  if (colonIdx < 0) return name
  const base = name.slice(0, colonIdx)
  const tag = name.slice(colonIdx + 1)
  if (tag.length <= 10) return name
  return `${base}:${tag.slice(0, 8)}…`
}

function GenerateReport() {
  const [selectedType, setSelectedType] = useState('standard')
  const [startDate, setStartDate] = useState(formatDateForInput(new Date()))
  const [endDate, setEndDate] = useState(formatDateForInput(new Date()))
  const [report, setReport] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const [models, setModels] = useState<OllamaModelInfo[]>([])
  const [selectedModel, setSelectedModel] = useState<string>(DEFAULT_MODEL_NAME)
  const [loadingModels, setLoadingModels] = useState(false)
  const [modelLoadError, setModelLoadError] = useState<string>('')
  const [dropdownOpen, setDropdownOpen] = useState(false)
  const [savedDefaultTip, setSavedDefaultTip] = useState<string>('')
  const dropdownRef = useRef<HTMLDivElement | null>(null)

  const loadModels = async (silent = false) => {
    if (!silent) setLoadingModels(true)
    setModelLoadError('')
    try {
      const list = await api.listOllamaModels()
      setModels(list)
      // 若无模型，则至少保留一个默认占位
      if (list.length === 0) {
        setSelectedModel(DEFAULT_MODEL_NAME)
        return
      }
      // 若当前选中的不在列表中，尝试切到第一个
      const exists = list.some((m) => m.name === selectedModel)
      if (!exists) {
        // 再尝试从配置里读默认模型
        try {
          const cfg = await api.getAppConfig()
          const configured = cfg?.ollama_model
          if (configured) {
            const inList = list.some((m) => m.name === configured)
            setSelectedModel(inList ? configured : list[0].name)
          } else {
            setSelectedModel(list[0].name)
          }
        } catch {
          setSelectedModel(list[0].name)
        }
      }
    } catch (err) {
      setModelLoadError(String(err))
      // 加载失败也保留一个默认占位，保证可操作
      if (models.length === 0) {
        setModels([{ name: DEFAULT_MODEL_NAME, model: DEFAULT_MODEL_NAME }])
      }
    } finally {
      if (!silent) setLoadingModels(false)
    }
  }

  // 首次挂载：先从配置读默认模型名，再拉取 Ollama 模型列表
  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const cfg = await api.getAppConfig()
        if (!cancelled && cfg?.ollama_model) {
          setSelectedModel(cfg.ollama_model)
        }
      } catch {
        // 忽略读取配置失败，继续用默认
      }
      if (!cancelled) {
        await loadModels(true)
      }
    })()
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // 点击下拉框外部关闭
  useEffect(() => {
    if (!dropdownOpen) return
    const handler = (e: MouseEvent) => {
      if (!dropdownRef.current) return
      if (!dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [dropdownOpen])

  const handleSetAsDefault = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      const current = await api.getAppConfig()
      await api.saveAppConfig({
        ...(current || {}),
        ollama_model: selectedModel,
      })
      setSavedDefaultTip(`已设为默认：${selectedModel}`)
      setTimeout(() => setSavedDefaultTip(''), 2500)
    } catch (err) {
      setSavedDefaultTip(`设置失败：${String(err)}`)
      setTimeout(() => setSavedDefaultTip(''), 3000)
    }
  }

  const handleGenerate = async () => {
    if (startDate > endDate) {
      setError('开始日期不能晚于结束日期')
      return
    }

    setLoading(true)
    setError('')
    setReport('')

    try {
      const modelArg = selectedModel && selectedModel !== DEFAULT_MODEL_NAME ? selectedModel : undefined
      // —— Step 1：快速健康检查（~2s），尽早发现 Ollama 未启动/模型未 pull ——
      try {
        await api.probeOllamaReady(modelArg)
      } catch (probeErr) {
        // 健康检查失败，直接把错误给用户（此时还未进入漫长的生成流程）
        setError(String(probeErr))
        setLoading(false)
        return
      }
      // —— Step 2：健康检查通过后再真正调用 AI 生成（耗时可能几十秒到 5 分钟）——
      const [_id, content] = await api.createAndSaveReport(selectedType, startDate, endDate, modelArg)
      setReport(content)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(report)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleQuickRange = (days: number) => {
    const end = new Date()
    const start = new Date()
    start.setDate(start.getDate() - days)
    setStartDate(formatDateForInput(start))
    setEndDate(formatDateForInput(end))
  }

  return (
    <div className="generate-report">
      <div className="gr-header">
        <div className="gr-header-left">
          <h1>生成报告</h1>
          <span className="gr-subtitle">基于本地 Ollama AI 生成工作报告</span>
        </div>
        <div className="gr-model-select" ref={dropdownRef}>
          <button
            type="button"
            className={`gr-model-trigger ${dropdownOpen ? 'open' : ''}`}
            onClick={() => setDropdownOpen((v) => !v)}
            title={selectedModel}
          >
            <span className="gr-model-dot" />
            <span className="gr-model-name">{shortModelName(selectedModel)}</span>
            <span className={`gr-model-caret ${dropdownOpen ? 'up' : ''}`}>▾</span>
          </button>

          {savedDefaultTip && <div className="gr-model-tip">{savedDefaultTip}</div>}

          {dropdownOpen && (
            <div className="gr-model-dropdown" role="listbox">
              <div className="gr-model-dropdown-header">
                <span>可用模型</span>
                <button
                  type="button"
                  className="gr-model-refresh-btn"
                  onClick={() => loadModels(false)}
                  disabled={loadingModels}
                  title="刷新模型列表"
                >
                  {loadingModels ? '刷新中…' : '⟳ 刷新'}
                </button>
              </div>

              {modelLoadError && (
                <div className="gr-model-error" title={modelLoadError}>
                  ⚠️ 加载模型失败：{String(modelLoadError).slice(0, 60)}
                </div>
              )}

              {models.length === 0 && !loadingModels ? (
                <div className="gr-model-empty">
                  暂无可选模型，请先在 Ollama 中 pull 至少一个模型
                </div>
              ) : (
                <ul className="gr-model-list">
                  {models.map((m) => (
                    <li
                      key={m.name}
                      role="option"
                      aria-selected={selectedModel === m.name}
                      className={`gr-model-option ${selectedModel === m.name ? 'active' : ''}`}
                      onClick={() => {
                        setSelectedModel(m.name)
                        setDropdownOpen(false)
                      }}
                      title={`${m.name}${m.size_human ? ` · ${m.size_human}` : ''}`}
                    >
                      <span className="gr-model-option-main">
                        <span className="gr-model-option-name">{m.name}</span>
                        {m.parameter_size && (
                          <span className="gr-model-option-tag">{m.parameter_size}</span>
                        )}
                      </span>
                      {(m.family || m.size_human) && (
                        <span className="gr-model-option-sub">
                          {m.family}
                          {m.size_human ? ` · ${m.size_human}` : ''}
                        </span>
                      )}
                      {selectedModel === m.name && <span className="gr-model-check">✓</span>}
                    </li>
                  ))}
                </ul>
              )}

              <div className="gr-model-dropdown-footer">
                <button
                  type="button"
                  className="gr-model-default-btn"
                  onClick={handleSetAsDefault}
                  title="将当前选中的模型设为默认（后续生成报告自动使用）"
                >
                  📌 设为默认模型
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="gr-content">
        <div className="gr-config-section">
          <div className="gr-section-title">
            <span className="tw-section-bar" />
            <h2>报告类型</h2>
          </div>
          <div className="gr-type-grid">
            {REPORT_TYPES.map((type) => (
              <div
                key={type.id}
                className={`gr-type-card ${selectedType === type.id ? 'selected' : ''}`}
                onClick={() => setSelectedType(type.id)}
              >
                <span className="gr-type-icon">{type.icon}</span>
                <div className="gr-type-info">
                  <span className="gr-type-name">{type.name}</span>
                  <span className="gr-type-desc">{type.desc}</span>
                </div>
                <span className={`gr-type-check ${selectedType === type.id ? 'show' : ''}`}>✓</span>
              </div>
            ))}
          </div>
        </div>

        <div className="gr-config-section">
          <div className="gr-section-title">
            <span className="tw-section-bar" />
            <h2>时间范围</h2>
          </div>
          <div className="gr-date-row">
            <div className="gr-date-item">
              <label>开始日期</label>
              <input type="date" value={startDate} max={endDate} onChange={(e) => setStartDate(e.target.value)} />
            </div>
            <div className="gr-date-item">
              <label>结束日期</label>
              <input type="date" value={endDate} max={formatDateForInput(new Date())} onChange={(e) => setEndDate(e.target.value)} />
            </div>
            <div className="gr-quick-range">
              <button className="gr-quick-btn" onClick={() => handleQuickRange(0)}>今天</button>
              <button className="gr-quick-btn" onClick={() => handleQuickRange(6)}>近7天</button>
              <button className="gr-quick-btn" onClick={() => handleQuickRange(29)}>近30天</button>
            </div>
          </div>
        </div>

        <div className="gr-action-row">
          <button
            className={`gr-generate-btn ${loading ? 'loading' : ''}`}
            onClick={handleGenerate}
            disabled={loading}
          >
            {loading ? (
              <>
                <span className="gr-spinner" />
                <span>AI 生成中...</span>
              </>
            ) : (
              <>
                <span>🚀</span>
                <span>生成报告</span>
              </>
            )}
          </button>
        </div>

        {error && (
          <div className="gr-error">
            <span className="gr-error-icon">⚠️</span>
            <span>{error}</span>
          </div>
        )}

        {report && (
          <div className="gr-report-section">
            <div className="gr-report-header">
              <div className="gr-section-title">
                <span className="tw-section-bar" />
                <h2>报告内容</h2>
              </div>
              <button className="gr-copy-btn" onClick={handleCopy}>
                {copied ? '✓ 已复制' : '📋 复制'}
              </button>
            </div>
            <div className="gr-report-body">
              <pre>{report}</pre>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

export default GenerateReport