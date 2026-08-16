import { useState } from 'react'
import { api } from '../utils/api'
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

function formatDateForInput(date: Date): string {
  return date.toISOString().split('T')[0]
}

function GenerateReport() {
  const [selectedType, setSelectedType] = useState('standard')
  const [startDate, setStartDate] = useState(formatDateForInput(new Date()))
  const [endDate, setEndDate] = useState(formatDateForInput(new Date()))
  const [report, setReport] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const handleGenerate = async () => {
    if (startDate > endDate) {
      setError('开始日期不能晚于结束日期')
      return
    }

    setLoading(true)
    setError('')
    setReport('')

    try {
      const [_id, content] = await api.createAndSaveReport(selectedType, startDate, endDate)
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
        <div className="gr-model-badge">
          <span className="gr-model-dot" />
          <span>qwen3:4b-fp16</span>
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