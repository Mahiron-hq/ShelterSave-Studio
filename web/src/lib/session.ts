const DB = "sss"
const STORE = "kv"

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1)
    req.onupgradeneeded = () => req.result.createObjectStore(STORE)
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function tx<T>(mode: IDBTransactionMode, run: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open()
  try {
    return await new Promise<T>((resolve, reject) => {
      const t = db.transaction(STORE, mode)
      const req = run(t.objectStore(STORE))
      t.oncomplete = () => resolve(req.result)
      t.onerror = () => reject(t.error)
      t.onabort = () => reject(t.error)
    })
  } finally {
    db.close()
  }
}

export interface Saved {
  session: string
  fileName: string
  savedAt: number
}

export async function readSession(): Promise<Saved | null> {
  try {
    const v = await tx("readonly", (s) => s.get("session"))
    return v && typeof v === "object" && typeof (v as Saved).session === "string" ? (v as Saved) : null
  } catch {
    return null
  }
}

export async function writeSession(v: Saved): Promise<boolean> {
  try {
    await tx("readwrite", (s) => s.put(v, "session"))
    return true
  } catch {
    return false
  }
}

export async function clearSession(): Promise<void> {
  try {
    await tx("readwrite", (s) => s.delete("session"))
  } catch {
    // Nothing to clear.
  }
}

export async function clearAll(): Promise<void> {
  try {
    localStorage.clear()
  } catch {
    // Storage blocked.
  }
  await new Promise<void>((resolve) => {
    const req = indexedDB.deleteDatabase(DB)
    req.onsuccess = req.onerror = req.onblocked = () => resolve()
  })
}
