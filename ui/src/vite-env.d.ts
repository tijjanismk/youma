/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Application Android compilée (fiche 0041) : « client », « livreur », ou absent sur le web. */
  readonly VITE_APPLI?: string;
}
