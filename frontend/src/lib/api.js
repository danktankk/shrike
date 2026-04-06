// frontend/src/lib/api.js
const BASE = '/api'

async function request(method, path, body) {
  const opts = {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  }
  const res = await fetch(BASE + path, opts)
  if (!res.ok) throw new Error(`${method} ${path} → ${res.status}`)
  if (res.status === 204) return null
  return res.json()
}

export const api = {
  searchTerms: {
    list: () => request('GET', '/search_terms'),
    create: (data) => request('POST', '/search_terms', data),
    update: (id, data) => request('PUT', `/search_terms/${id}`, data),
    delete: (id) => request('DELETE', `/search_terms/${id}`),
  },
  sources: {
    list: () => request('GET', '/sources'),
    create: (data) => request('POST', '/sources', data),
    update: (id, data) => request('PUT', `/sources/${id}`, data),
    delete: (id) => request('DELETE', `/sources/${id}`),
    test: (id) => request('POST', `/sources/${id}/test`),
  },
  matches: {
    list: (params = {}) => {
      const qs = new URLSearchParams(Object.entries(params).filter(([,v]) => v != null))
      return request('GET', `/matches${qs.toString() ? '?' + qs : ''}`)
    },
  },
  notifications: {
    getConfig: () => request('GET', '/notifications/config'),
    test: (channel) => request('POST', `/notifications/test/${channel}`),
  },
}
