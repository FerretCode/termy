export interface RunCell {
  id: string
  value: string
  currentDir: string
}

export interface XtermSize {
  rows: number
  cols: number
}

export interface FrontendMessage {
  id: string
  action: 'resume' | 'kill' | { write: string } | { resize: XtermSize }
}

export type Message =
  | { type: 'api'; command: string } // todo: create types for the api
  | { type: 'get-suggestions'; value: string; currentDir: string }
  | ({ type: 'run-cell' } & RunCell)
  | ({ type: 'frontend-message' } & FrontendMessage)
  | { type: 'write'; path: string; value: string }
  | { type: 'get-window-info' }
  | { type: 'window-action'; action: WindowAction }

export type WindowAction = 'minimize' | 'maximize' | 'unmaximize' | 'close'

export interface ServerMessage {
  action?: [ActionKeys, string][]
  text?: number[]
  mdx?: string
  api?: string
  status?: Status
  error?: string
}

export type ActionKeys = 'cd' | 'theme' | 'pretty_path' | 'branch'

export interface Suggestion {
  label: string
  kind: SuggestionKind
  insertText?: string // defaults to label if not defined
  documentation?: string
}

export interface NativeSuggestion extends Suggestion {
  score: number
  date?: string
}

export type SuggestionKind =
  | 'file'
  | 'directory'
  | 'executable'
  | 'history'
  | 'externalHistory'

export interface WindowInfo {
  isMaximized: boolean
}
