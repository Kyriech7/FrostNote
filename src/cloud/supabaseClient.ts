import { createClient } from "@supabase/supabase-js";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL as string | undefined;
const supabaseKey = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY as string | undefined;

export const isCloudConfigured = Boolean(supabaseUrl && supabaseKey);

export const supabase = isCloudConfigured
  ? createClient(supabaseUrl as string, supabaseKey as string, {
      auth: {
        autoRefreshToken: true,
        persistSession: true,
      },
    })
  : null;
