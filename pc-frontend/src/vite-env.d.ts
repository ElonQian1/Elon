/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PC_LEGACY_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

// CSS Modules
declare module '*.module.css' {
  const classes: Record<string, string>
  export default classes
}
