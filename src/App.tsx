import { BrowserRouter, Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import Dashboard from './pages/Dashboard'
import Daily from './pages/Daily'
import Weekly from './pages/Weekly'
import Monthly from './pages/Monthly'

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Dashboard />} />
          <Route path="daily" element={<Daily />} />
          <Route path="weekly" element={<Weekly />} />
          <Route path="monthly" element={<Monthly />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App