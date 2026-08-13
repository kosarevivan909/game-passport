export class ProfileLimitError extends Error {
  constructor() { super("You can save up to 5 profiles."); this.name = "ProfileLimitError"; }
}

export class ConfigurationError extends Error {
  constructor(message: string) { super(message); this.name = "ConfigurationError"; }
}
