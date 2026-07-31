const EXACT_LABELS = {
  RECOMMENDED: 'Recommended',
  ALLOWED_WITH_CAVEATS: 'Allowed with caveats',
  FRAGILE: 'Fragile',
  PRACTICALLY_UNAVAILABLE: 'Practically unavailable',
  LEGALLY_BLOCKED: 'Legally blocked',
  UNKNOWN: 'Unknown',
  HIGHLY_CONSTRAINED: 'Highly constrained',
  TRANSITIONAL: 'Transitional',
  MANAGED_ACCESS_AVAILABLE: 'Managed access available',
  COST_CONSTRAINED: 'Cost constrained',
  ComprehensivelySanctioned: 'Comprehensively sanctioned',
  PostSanctionsLag: 'Post-sanctions lag',
  UsComputeStack: 'US compute stack',
  ResourceConstrained: 'Resource constrained',
  Government: 'Government',
  Ngo: 'NGO',
  Private: 'Private sector',
  Academic: 'Academic',
  Public: 'Public',
  Internal: 'Internal',
  Sensitive: 'Sensitive',
  Classified: 'Classified',
  compute: 'Compute',
  data: 'Data',
  operational: 'Operational',
  infrastructure: 'Infrastructure',
  network: 'Network',
  payment: 'Payment',
  provider: 'Provider',
  privacy: 'Privacy',
  service_channel: 'Service channel',
  local_open_weight: 'Fully local open-weight deployment',
  local_small_model: 'Local small model on workstation hardware',
  sovereign_vpc_open_weight: 'Sovereign VPC with open-weight model',
  us_frontier_api: 'US frontier API',
  selective_cloud_fallback: 'Selective cloud fallback',
  circumvention_mediated_access: 'Circumvention-mediated access',
  local_compute: 'Local compute',
  workstation_compute: 'Workstation compute',
  model_weights_download: 'Model weights download',
  operator_control: 'Operator control',
  country_controlled_vpc: 'Country-controlled VPC',
  internet_connectivity: 'Internet connectivity',
  cross_border_billing: 'Cross-border billing',
  adaptive_connectivity: 'Adaptive connectivity',
  openai_api: 'OpenAI API',
  claude_api: 'Claude API',
  huggingface_weights: 'Hugging Face weights',
  signal: 'Signal',
  tor: 'Tor',
}

const CONTEXT_LABELS = {
  'constraint-target:tor': 'Tor circumvention resilience',
  'constraint-target:signal': 'Signal reachability',
  'constraint-target:huggingface_weights': 'Hugging Face weights reachability',
}

export function humanize(value, context = 'default') {
  if (value == null || value === '') return '—'

  const raw = String(value)
  const contextLabel = CONTEXT_LABELS[`${context}:${raw}`]
  if (contextLabel) return contextLabel
  if (EXACT_LABELS[raw]) return EXACT_LABELS[raw]

  let result = raw
  for (const [from, to] of Object.entries(EXACT_LABELS).sort((a, b) => b[0].length - a[0].length)) {
    result = result.replaceAll(from, to)
  }

  if (result !== raw) return result

  const normalized = raw
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replaceAll('_', ' ')
    .replaceAll('-', ' ')
    .trim()

  return normalized.replace(/\b\w/g, (char) => char.toUpperCase())
}
