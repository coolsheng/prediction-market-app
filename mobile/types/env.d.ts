declare namespace NodeJS {
  interface ProcessEnv {
    EXPO_PUBLIC_API_KEY?: string;
    EXPO_PUBLIC_BACKEND_URL?: string;
  }
}

declare const process: {
  env: NodeJS.ProcessEnv;
};
