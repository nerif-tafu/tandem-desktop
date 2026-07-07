export interface PresentationWindow {
  id: string;
  label: string;
}

export interface PresentationExtensionStatus {
  applicable: boolean;
  installed: boolean;
  enabled: boolean;
  active: boolean;
  needsLogout: boolean;
  message?: string;
}
