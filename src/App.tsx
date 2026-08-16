import { BrowserRouter, Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import TodayWork from './pages/TodayWork'
import GenerateReport from './pages/GenerateReport'
import WorkTimeline from './pages/WorkTimeline'
import TimeHeatmap from './pages/TimeHeatmap'
import AppRecords from './pages/AppRecords'
import HistoryReports from './pages/HistoryReports'
import Privacy from './pages/Privacy'
import Settings from './pages/Settings'

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<TodayWork />} />
          <Route path="generate-report" element={<GenerateReport />} />
          <Route path="work-timeline" element={<WorkTimeline />} />
          <Route path="time-heatmap" element={<TimeHeatmap />} />
          <Route path="app-records" element={<AppRecords />} />
          <Route path="history-reports" element={<HistoryReports />} />
          <Route path="privacy" element={<Privacy />} />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App