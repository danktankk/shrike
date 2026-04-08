// frontend/src/lib/api.js
const BASE = '/api'

async function request(method, path, body) {
  const opts = {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  }
  const res = await fetch(BASE + path, opts)
  if (!res.ok) {
    let msg = `${method} ${path} → ${res.status}`
    try {
      const body = await res.json()
      msg = body.error || body.message || msg
    } catch {}
    throw new Error(msg)
  }
  if (res.status === 204) return null
  return res.json()
}

export const api = {
  searchTerms: {
    list: () => request('GET', '/search_terms'),
    create: (data) => request('POST', '/search_terms', data),
    update: (id, data) => request('PUT', `/search_terms/${id}`, data),
    delete: (id) => request('DELETE', `/search_terms/${id}`),
    scan: (id) => request('POST', `/search_terms/${id}/scan`),
  },
  sources: {
    list: () => request('GET', '/sources'),
    create: (data) => request('POST', '/sources', data),
    update: (id, data) => request('PUT', `/sources/${id}`, data),
    delete: (id) => request('DELETE', `/sources/${id}`),
    test: (id) => request('POST', `/sources/${id}/test`),
    categories: (id) => request('GET', `/sources/${id}/categories`),
  },
  matches: {
    list: (params = {}) => {
      const qs = new URLSearchParams(Object.entries(params).filter(([,v]) => v != null))
      return request('GET', `/matches${qs.toString() ? '?' + qs : ''}`)
    },
    get: (id) => request('GET', `/match/${id}`),
    delete: (id) => request('DELETE', `/matches/${id}`),
    clearAll: () => request('DELETE', '/matches'),
  },
  notifications: {
    getConfig: () => request('GET', '/notifications/config'),
    test: (channel) => request('POST', `/notifications/test/${channel}`),
  },
  scan: () => request('POST', '/scan'),
  // GET /api/art?q=<title> → { found, game: GameRef|null }
  art: (q) => request('GET', `/art?q=${encodeURIComponent(q)}`),
}
