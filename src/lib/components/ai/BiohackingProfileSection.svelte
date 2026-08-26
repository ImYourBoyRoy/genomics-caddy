<!-- ./src/lib/components/ai/BiohackingProfileSection.svelte -->
<script lang="ts">
  import {
    EMPTY_PERSONAL_DIETARY_PROFILE,
    parseContextList,
    savePersonalSafetyContext,
    type PersonalDietaryProfile,
    type PersonalSafetyContext,
  } from '../../utils/personalSafetyContext';
  import ReproductiveContextEditor from './ReproductiveContextEditor.svelte';

  interface UserBiohackingProfile {
    goals: string;
    challenges: string;
    relevantBodySystems: string;
    reproductiveHormoneContext: string;
    diet: string;
    supplements: string;
    medications: string;
    bloodwork: string;
    diagnoses: string;
    supportiveTests: string;
    injectProfile: boolean;
  }

  interface Props {
    userProfile: UserBiohackingProfile;
    personalSafetyContext: PersonalSafetyContext;
    reproductiveContext?: string;
    sampleId?: number;
  }

  type ContextListField = 'medications' | 'supplements' | 'allergies' | 'symptoms' | 'labObservations';
  type DietaryProfileField = keyof PersonalDietaryProfile;

  let {
    userProfile = $bindable(),
    personalSafetyContext = $bindable(),
    reproductiveContext = '',
    sampleId,
  }: Props = $props();

  function contextText(field: ContextListField): string {
    return personalSafetyContext[field].join('\n');
  }

  function updateContextList(field: ContextListField, event: Event): void {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    personalSafetyContext[field] = parseContextList(value);
    savePersonalSafetyContext(sampleId, personalSafetyContext);
  }

  function dietaryProfileText(field: DietaryProfileField): string {
    return personalSafetyContext.dietaryProfile?.[field]?.join('\n') || '';
  }

  function updateDietaryProfile(field: DietaryProfileField, event: Event): void {
    const value = (event.currentTarget as HTMLTextAreaElement).value;
    const nextProfile: PersonalDietaryProfile = {
      ...EMPTY_PERSONAL_DIETARY_PROFILE,
      ...(personalSafetyContext.dietaryProfile || {}),
      [field]: parseContextList(value),
    };
    const hasValues = Object.values(nextProfile).some((items) => items.length > 0);
    if (hasValues) {
      personalSafetyContext.dietaryProfile = nextProfile;
    } else {
      const { dietaryProfile: _removed, ...withoutDietaryProfile } = personalSafetyContext;
      personalSafetyContext = withoutDietaryProfile;
    }
    savePersonalSafetyContext(sampleId, personalSafetyContext);
  }
</script>

<div class="settings-details-content" style="gap: 10px; padding: 0;">
  <label class="extended-thinking-toggle">
    <input type="checkbox" bind:checked={userProfile.injectProfile} />
    <span class="highlight-text font-bold">Inject profile into AI context</span>
  </label>

  <div class="input-row">
    <label for="profile-goals">Goals &amp; Objectives</label>
    <textarea id="profile-goals" bind:value={userProfile.goals} placeholder="e.g. optimize methylation, fix afternoon fatigue..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-challenges">Current Challenges &amp; Symptoms</label>
    <textarea id="profile-challenges" bind:value={userProfile.challenges} placeholder="e.g. brain fog, caffeine sensitivity, insomnia..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-body-systems">Relevant Body Systems &amp; Life Context (Optional)</label>
    <textarea id="profile-body-systems" bind:value={userProfile.relevantBodySystems} placeholder="e.g. menstrual/reproductive, prostate/testicular, thyroid, cardiovascular, unknown..." class="profile-textarea"></textarea>
    <small>Use this to choose relevant biology; the app does not infer anatomy or identity from DNA.</small>
  </div>
  <div class="input-row">
    <label for="profile-reproductive-context">Reproductive &amp; Hormone Context (Optional)</label>
    <textarea id="profile-reproductive-context" bind:value={userProfile.reproductiveHormoneContext} placeholder="e.g. cycle timing, contraception name/ingredients, menopause, pregnancy/postpartum, hormone therapy, none/unknown..." class="profile-textarea"></textarea>
    <small>Exact medication names and active ingredients are more useful than a genetic-sex label.</small>
  </div>

  <ReproductiveContextEditor bind:personalSafetyContext {reproductiveContext} {sampleId} />

  <div class="input-row">
    <label for="profile-diet">Average Diet</label>
    <textarea id="profile-diet" bind:value={userProfile.diet} placeholder="e.g. low carb, high protein, vegetarian, average diet..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-supplements">Supplement List</label>
    <textarea id="profile-supplements" bind:value={userProfile.supplements} placeholder="e.g. Vitamin D3, magnesium, omega-3..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-medications">Medications List</label>
    <textarea id="profile-medications" bind:value={userProfile.medications} placeholder="e.g. levothyroxine, none..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-bloodwork">Blood Work History (With Dates)</label>
    <textarea id="profile-bloodwork" bind:value={userProfile.bloodwork} placeholder="e.g. Fasting glucose 95 mg/dL (2025-10-15)..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-diagnoses">Diagnosis List</label>
    <textarea id="profile-diagnoses" bind:value={userProfile.diagnoses} placeholder="e.g. ADHD, hypothyroid..." class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="profile-supportive">Supportive Tests</label>
    <textarea id="profile-supportive" bind:value={userProfile.supportiveTests} placeholder="e.g. Sleep study showing mild apnea..." class="profile-textarea"></textarea>
  </div>

  <div class="structured-context-divider">
    <strong>Structured safety context for this DNA profile</strong>
    <small>One item per line. These entries are self-reported context, not DNA findings. Exact names, active ingredients, dates, units, and reference ranges improve safety review.</small>
  </div>
  <div class="input-row">
    <label for="context-medications">Current medications</label>
    <textarea id="context-medications" value={contextText('medications')} oninput={(event) => updateContextList('medications', event)} placeholder="e.g. exact birth-control product and active ingredient; levothyroxine" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="context-supplements">Current supplements / OTC products</label>
    <textarea id="context-supplements" value={contextText('supplements')} oninput={(event) => updateContextList('supplements', event)} placeholder="e.g. magnesium glycinate; fish oil; multivitamin" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="context-allergies">Allergies / intolerances</label>
    <textarea id="context-allergies" value={contextText('allergies')} oninput={(event) => updateContextList('allergies', event)} placeholder="e.g. fish; penicillin; lactose intolerance" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="context-symptoms">Symptoms and timing</label>
    <textarea id="context-symptoms" value={contextText('symptoms')} oninput={(event) => updateContextList('symptoms', event)} placeholder="e.g. pelvic pain; heavy bleeding; mood changes in the late luteal phase" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="context-labs">Recent labs / clinician findings</label>
    <textarea id="context-labs" value={contextText('labObservations')} oninput={(event) => updateContextList('labObservations', event)} placeholder="e.g. ferritin 18 ng/mL (2026-05-10), reference range ..." class="profile-textarea"></textarea>
  </div>

  <div class="structured-context-divider">
    <strong>Explicit food requirements (optional)</strong>
    <small>These are user-entered food rules, not DNA findings. Use confirmed versus suspected allergy fields carefully; never use a consumer DNA result to clear an allergen or diagnose celiac disease.</small>
  </div>
  <div class="input-row">
    <label for="food-hard-exclusions">Foods or ingredients to exclude</label>
    <textarea id="food-hard-exclusions" value={dietaryProfileText('hard_exclusions')} oninput={(event) => updateDietaryProfile('hard_exclusions', event)} placeholder="e.g. pork; alcohol; coffee" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-confirmed-allergies">Confirmed food allergies</label>
    <textarea id="food-confirmed-allergies" value={dietaryProfileText('allergies_confirmed')} oninput={(event) => updateDietaryProfile('allergies_confirmed', event)} placeholder="e.g. peanut; fish; milk" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-suspected-allergies">Suspected food reactions / intolerances</label>
    <textarea id="food-suspected-allergies" value={dietaryProfileText('allergies_suspected')} oninput={(event) => updateDietaryProfile('allergies_suspected', event)} placeholder="e.g. lactose symptoms; possible sesame reaction" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-religious-profiles">Religious / cultural food profiles</label>
    <textarea id="food-religious-profiles" value={dietaryProfileText('religious_cultural_profiles')} oninput={(event) => updateDietaryProfile('religious_cultural_profiles', event)} placeholder="e.g. halal_compatible; kosher_style; pork_free" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-ethical-profiles">Ethical food profiles</label>
    <textarea id="food-ethical-profiles" value={dietaryProfileText('ethical_preference_profiles')} oninput={(event) => updateDietaryProfile('ethical_preference_profiles', event)} placeholder="e.g. vegetarian; vegan; pescatarian" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-medical-profiles">Clinician-confirmed or explicitly discussed medical diet profiles</label>
    <textarea id="food-medical-profiles" value={dietaryProfileText('medical_diet_profiles')} oninput={(event) => updateDietaryProfile('medical_diet_profiles', event)} placeholder="e.g. celiac_gluten_free; low_sodium; low_iron_support" class="profile-textarea"></textarea>
  </div>
  <div class="input-row">
    <label for="food-goals">Food or nutrition goals</label>
    <textarea id="food-goals" value={dietaryProfileText('goals')} oninput={(event) => updateDietaryProfile('goals', event)} placeholder="e.g. PMDD_cycle; muscle_gain; gut_symptoms" class="profile-textarea"></textarea>
  </div>
</div>

<style>
  .input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-row label {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .structured-context-divider {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border-color);
  }

  .structured-context-divider strong {
    color: var(--text-primary);
    font-size: 0.8rem;
  }

  .structured-context-divider small {
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }

  .profile-textarea {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 8px;
    border-radius: 6px;
    font-size: 0.8rem;
    min-height: 52px;
    resize: vertical;
    font-family: inherit;
    line-height: 1.4;
  }
  
  .profile-textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .extended-thinking-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--text-primary);
  }
  
  .extended-thinking-toggle input {
    cursor: pointer;
  }

  .highlight-text {
    color: var(--text-primary);
  }

  .font-bold {
    font-weight: bold;
  }
</style>
