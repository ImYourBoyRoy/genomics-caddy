// ./src/lib/utils/layperson.ts
/**
 * Purpose: Provide clear, plain-English translations for genetic marker impacts and meanings.
 * Responsibilities:
 * - Define the LaypersonTranslation interface.
 * - Map all 99 rsIDs from markers_extracted.json to their simplified layperson-friendly translations.
 * Key Inputs: None (static mapping).
 * Key Outputs: LAYPERSON_MAP dictionary.
 * Operational Notes: Keeps technical and medical validity while using terms a layperson or teenager can understand.
 */

export interface LaypersonTranslation {
  simpleImpact: string;
  simpleMeaning: string;
}

export const LAYPERSON_MAP: Record<string, LaypersonTranslation> = {
  "rs80357159": {
    simpleImpact: "Altered DNA repair (BRCA1 gene)",
    simpleMeaning: "This is a research-only marker for DNA repair. It requires a doctor's confirmation and more clinical testing before making any health decisions."
  },
  "rs80357224": {
    simpleImpact: "Altered DNA repair (BRCA1 gene)",
    simpleMeaning: "This is a research-only marker for DNA repair. It requires a doctor's confirmation and more clinical testing before making any health decisions."
  },
  "rs11571833": {
    simpleImpact: "Harmless DNA repair variant (BRCA2 gene)",
    simpleMeaning: "This gene change is considered harmless and does not indicate a classic hereditary cancer risk. It is a common variant used in research."
  },
  "rs429358": {
    simpleImpact: "APOE e4 variant linked to cholesterol and brain health",
    simpleMeaning: "This gene variant affects how your body handles fats. Having one or two copies of this variant can lead to higher cholesterol levels and a higher long-term risk of heart issues or memory decline."
  },
  "rs7412": {
    simpleImpact: "APOE gene type determination",
    simpleMeaning: "This marker determines your specific APOE gene type. It helps show whether your body is naturally prone to lower or higher cholesterol levels."
  },
  "rs10757278": {
    simpleImpact: "Increased risk of early-onset heart disease (9p21 region)",
    simpleMeaning: "This variant affects cells in your blood vessel walls. Certain versions are linked to a higher risk of early heart attacks, independent of your cholesterol levels."
  },
  "rs1799983": {
    simpleImpact: "Reduced nitric oxide production (NOS3 gene)",
    simpleMeaning: "You may produce less nitric oxide, which helps widen blood vessels. This can affect how well your blood flows, your exercise recovery, and how your blood vessels react to stress."
  },
  "rs2070744": {
    simpleImpact: "Lower nitric oxide levels (NOS3 gene)",
    simpleMeaning: "This variant lowers the production of nitric oxide in your blood vessels. Working with other markers, it can make it harder for your blood vessels to relax."
  },
  "rs1800012": {
    simpleImpact: "Slightly reduced bone density and collagen strength (COL1A1 gene)",
    simpleMeaning: "This variant can slightly lower the strength of your body's main structural support, collagen. It is linked to slightly weaker bones and a higher chance of soft-tissue injuries like sprains."
  },
  "rs12722": {
    simpleImpact: "Tendon and ligament flexibility (COL5A1 gene)",
    simpleMeaning: "This variant affects the structure of your tendons and ligaments. Some versions give you more joint flexibility, while others make you more prone to tendon strains."
  },
  "rs1800255": {
    simpleImpact: "Common collagen structure variant (COL3A1 gene)",
    simpleMeaning: "This is a common, non-harmful change in your collagen structure. It does not cause serious tissue disease but plays a role in natural joint and tissue flexibility."
  },
  "rs143383": {
    simpleImpact: "Reduced joint cartilage repair (GDF5 gene)",
    simpleMeaning: "This variant reduces the production of a protein needed to repair joint cartilage. It can slightly increase your chances of developing joint wear-and-tear (osteoarthritis) over time, especially in the knees and hips."
  },
  "rs3025058": {
    simpleImpact: "Altered tissue remodeling in tendons (MMP3 gene)",
    simpleMeaning: "This variant affects how your body breaks down and rebuilds tissue in your joints and tendons. It can influence your risk of tendon injuries, such as Achilles tendon issues."
  },
  "rs2118181": {
    simpleImpact: "Altered artery wall elasticity (FBN1 gene)",
    simpleMeaning: "This variant affects the elasticity of your blood vessels. It is linked to a slight increase in risk for blood vessel stretching or tearing, but it is not a diagnosis of a genetic disorder like Marfan syndrome."
  },
  "rs8033037": {
    simpleImpact: "Common bone growth variant (FBN1 gene)",
    simpleMeaning: "This common variant is linked to natural variations in height and bone length. It is not harmful and represents normal human diversity."
  },
  "rs1815739": {
    simpleImpact: "Reduced fast-twitch muscle power (ACTN3 gene)",
    simpleMeaning: "Having two copies of this variant makes your body stop producing a muscle protein found in fast-twitch fibers. This shifts your natural athletic profile toward endurance and away from pure explosive power."
  },
  "rs970547": {
    simpleImpact: "Altered ligament strength (COL12A1 gene)",
    simpleMeaning: "This variant affects how collagen is organized in your ligaments. Certain versions are associated with a slightly higher risk of ligament tears, such as ACL injuries in the knee."
  },
  "rs3918242": {
    simpleImpact: "Altered tissue remodeling (MMP9 gene)",
    simpleMeaning: "This variant changes how your body maintains and breaks down tissues in your joints and blood vessels, which can impact your overall joint and heart health."
  },
  "rs10156191": {
    simpleImpact: "Reduced breakdown of histamine in food (AOC1 gene)",
    simpleMeaning: "You produce less of the enzyme that breaks down histamine in your gut. This can make you more sensitive to histamine-rich foods like aged cheeses, wine, or fermented items, leading to digestive issues or hives."
  },
  "rs11558538": {
    simpleImpact: "Slower breakdown of histamine in cells (HNMT gene)",
    simpleMeaning: "This variant slows down how your brain and body clear histamine from inside your cells. This can make histamine signals last longer, potentially affecting sleep, focus, or allergy-like symptoms."
  },
  "rs4988235": {
    simpleImpact: "Lactose intolerance (MCM6 gene)",
    simpleMeaning: "Having two copies of this variant makes your body stop producing lactase, the enzyme needed to digest milk sugar (lactose), as you grow older. This leads to lactose intolerance."
  },
  "rs9939609": {
    simpleImpact: "Increased appetite and slower fullness signals (FTO gene)",
    simpleMeaning: "This variant is linked to a naturally higher appetite and taking longer to feel full. Regular exercise and eating enough protein can help counteract this effect."
  },
  "rs1801282": {
    simpleImpact: "Altered fat storage and insulin sensitivity (PPARG gene)",
    simpleMeaning: "This variant helps control fat storage and how your body responds to insulin. Some versions improve your insulin response, which helps protect you against type 2 diabetes."
  },
  "rs2237897": {
    simpleImpact: "Lower insulin response to sugar (KCNQ1 gene)",
    simpleMeaning: "This variant can lower the amount of insulin your body releases in response to sugar. This slightly raises your risk of developing type 2 diabetes."
  },
  "rs7903146": {
    simpleImpact: "Elevated risk of type 2 diabetes (TCF7L2 gene)",
    simpleMeaning: "This is the strongest common genetic marker linked to type 2 diabetes. It reduces insulin secretion, but eating a low-sugar diet and staying active can significantly lower your risk."
  },
  "rs13266634": {
    simpleImpact: "Altered zinc transport in insulin cells (SLC30A8 gene)",
    simpleMeaning: "This variant affects how zinc is moved inside your insulin-producing cells. This can alter how insulin is stored and released, slightly increasing your risk of diabetes."
  },
  "rs5219": {
    simpleImpact: "Reduced insulin release (KCNJ11 gene)",
    simpleMeaning: "This variant keeps channels in your pancreas open longer, which slows down the release of insulin. This can slightly elevate your blood sugar response after meals."
  },
  "rs1260326": {
    simpleImpact: "Triglyceride and sugar balance shift (GCKR gene)",
    simpleMeaning: "This variant shifts how your liver handles sugars and fats. Certain versions lower your fasting blood sugar but increase your level of triglycerides (fats in the blood)."
  },
  "rs662799": {
    simpleImpact: "Reduced clearance of blood fats (APOA5 gene)",
    simpleMeaning: "This variant slows down how quickly your body clears fat from your blood. This can lead to higher levels of fasting triglycerides."
  },
  "rs328": {
    simpleImpact: "Faster clearance of blood fats (LPL gene)",
    simpleMeaning: "This variant increases the activity of an enzyme that clears fat from your blood. This is generally beneficial, leading to lower triglycerides and higher good (HDL) cholesterol."
  },
  "rs708272": {
    simpleImpact: "Increased good (HDL) cholesterol (CETP gene)",
    simpleMeaning: "This variant reduces the activity of a protein that moves cholesterol around. This leads to higher levels of protective 'good' HDL cholesterol in your blood."
  },
  "rs10455872": {
    simpleImpact: "Elevated Lipoprotein(a) levels (LPA gene)",
    simpleMeaning: "This marker is strongly linked to higher levels of a specific type of cholesterol-carrying particle called Lipoprotein(a). High levels can increase the risk of plaque buildup in your arteries."
  },
  "rs3798220": {
    simpleImpact: "Elevated Lipoprotein(a) levels (LPA gene)",
    simpleMeaning: "This variant causes your body to make a smaller version of a blood protein that is cleared more slowly. This raises your levels of Lipoprotein(a), a risk factor for heart disease."
  },
  "rs17782313": {
    simpleImpact: "Altered fullness signaling (MC4R gene)",
    simpleMeaning: "This variant affects how your brain receives signals about being full. Certain versions can make you feel less satisfied after eating, leading to a tendency to eat more."
  },
  "rs1360780": {
    simpleImpact: "Altered stress hormone sensitivity (FKBP5 gene)",
    simpleMeaning: "This variant affects how your brain turns off its stress response. Certain versions make you more sensitive to stress and can prolong the effects of the stress hormone cortisol."
  },
  "rs6311": {
    simpleImpact: "Altered serotonin receptor density (HTR2A gene)",
    simpleMeaning: "This research-level variant changes how many serotonin receptors are on your brain cells. It can influence your mood, emotional reactivity, and how you react to certain medications."
  },
  "rs53576": {
    simpleImpact: "Altered oxytocin (social hormone) sensitivity (OXTR gene)",
    simpleMeaning: "This variant can affect how sensitive your brain is to oxytocin, a hormone involved in bonding and social stress management. Certain versions can affect how you seek support or handle stress around others."
  },
  "rs6323": {
    simpleImpact: "Slower breakdown of brain chemicals (MAOA gene)",
    simpleMeaning: "This variant slows down how quickly your body clears brain chemicals like dopamine and serotonin. This can lead to stronger emotional responses under sudden stress."
  },
  "rs4680": {
    simpleImpact: "Slower dopamine breakdown in the brain (COMT gene)",
    simpleMeaning: "Having two copies of this variant makes your body clear dopamine from your brain very slowly. This helps with deep focus and planning, but can also make you more prone to anxiety and stress under pressure."
  },
  "rs1800497": {
    simpleImpact: "Fewer dopamine receptors (DRD2/ANKK1 gene region)",
    simpleMeaning: "This variant reduces the number of dopamine receptors in your brain's reward center. This can make you more likely to seek out strong rewards or stimulation."
  },
  "rs6277": {
    simpleImpact: "Altered dopamine receptor availability (DRD2 gene)",
    simpleMeaning: "This variant affects how efficiently your brain makes dopamine receptors, which can play a role in memory, learning, and mental flexibility."
  },
  "rs7794154": {
    simpleImpact: "Altered brain cell connections (CNTNAP2 gene)",
    simpleMeaning: "This research-level variant is involved in how brain cells connect and communicate. It has been studied in relation to language development and sensory sensitivity."
  },
  "rs4307059": {
    simpleImpact: "Altered brain structure connections (MSNP1AS gene)",
    simpleMeaning: "This is a minor research marker that plays a role in brain cell architecture. It shows a weak association with sensory processing and neurodevelopmental traits."
  },
  "rs1800544": {
    simpleImpact: "Altered adrenaline receptor sensitivity in the brain (ADRA2A gene)",
    simpleMeaning: "This variant changes how your brain responds to norepinephrine, a chemical related to focus and stress. It is often studied for its role in attention and focus medication response."
  },
  "rs1801132": {
    simpleImpact: "Estrogen receptor sensitivity (ESR1 gene)",
    simpleMeaning: "This variant affects how your cells respond to estrogen. It has been researched for its relationship with mood changes during hormonal shifts."
  },
  "rs2234693": {
    simpleImpact: "Estrogen binding sensitivity (ESR1 gene)",
    simpleMeaning: "This research marker affects how your body processes estrogen. It is studied for links to emotional and physical symptoms during hormone drops."
  },
  "rs8079626": {
    simpleImpact: "Progesterone receptor sensitivity (PGR gene)",
    simpleMeaning: "This marker affects how your cells respond to progesterone. It is researched in relation to mood fluctuations during menstrual cycle hormonal peaks."
  },
  "rs6265": {
    simpleImpact: "Reduced brain growth factor release (BDNF gene)",
    simpleMeaning: "This variant reduces the release of a key protein that helps brain cells grow and adapt. It is linked to differences in memory, learning, and how your brain benefits from cardio exercise."
  },
  "rs324420": {
    simpleImpact: "Higher natural 'bliss' chemical levels (FAAH gene)",
    simpleMeaning: "This variant slows down the breakdown of a natural brain chemical often called the 'bliss molecule.' It is linked to lower anxiety and a better ability to handle stress."
  },
  "rs5751876": {
    simpleImpact: "Increased risk of anxiety from caffeine (ADORA2A gene)",
    simpleMeaning: "This variant affects how your brain reacts to caffeine. Certain versions can cause a much stronger fight-or-flight response, leading to jitteriness and anxiety after drinking coffee or energy drinks."
  },
  "rs1799971": {
    simpleImpact: "Altered pain and reward sensitivity (OPRM1 gene)",
    simpleMeaning: "This variant affects your body's endorphin system. It can slightly change your pain tolerance and affect how you respond to rewards or social bonding."
  },
  "rs1611115": {
    simpleImpact: "Altered dopamine and norepinephrine balance (DBH gene)",
    simpleMeaning: "This variant reduces the speed at which your body turns dopamine into norepinephrine. This changes the balance of key focus and stress chemicals in your nervous system."
  },
  "rs1801133": {
    simpleImpact: "Folate-processing context marker (MTHFR gene)",
    simpleMeaning: "This common variant may modestly affect folate-related lab values in some people, especially with two copies, but it does not mean you cannot process folic acid. Do not avoid folic acid or replace it with a supplement solely because of this genotype; use diet, labs, pregnancy context, and clinician guidance."
  },
  "rs1801131": {
    simpleImpact: "Folate-processing context marker (MTHFR gene)",
    simpleMeaning: "This common variant is usually a small-effect context marker. It does not diagnose a folate problem or justify avoiding folic acid; consider folate, B12, and homocysteine only when clinically relevant."
  },
  "rs1800795": {
    simpleImpact: "Higher baseline inflammation (IL-6 gene)",
    simpleMeaning: "This variant is linked to higher baseline levels of an inflammatory protein. This can make you more sensitive to physical and mental stress when your immune system is active."
  },
  "rs1800629": {
    simpleImpact: "Increased inflammatory response (TNF gene)",
    simpleMeaning: "This variant makes your body produce more of a key inflammatory protein. This can raise your body's overall baseline level of inflammation."
  },
  "rs1800562": {
    simpleImpact: "Increased iron absorption risk (HFE gene)",
    simpleMeaning: "Having two copies of this variant puts you at high risk for absorbing too much iron from your diet, which can build up in your organs and require medical management."
  },
  "rs1799945": {
    simpleImpact: "Mildly increased iron absorption (HFE gene)",
    simpleMeaning: "This is a milder iron-absorption variant. On its own, it rarely causes issues, but when combined with other variants, it can lead to mild iron buildup."
  },
  "rs1805087": {
    simpleImpact: "Altered folate and B12 processing (MTR gene)",
    simpleMeaning: "This variant affects an enzyme that works with folate and vitamin B12. It can slightly alter how your body uses these vitamins for cell maintenance."
  },
  "rs1801394": {
    simpleImpact: "Slower Vitamin B12 recycling (MTRR gene)",
    simpleMeaning: "This variant makes your body recycle vitamin B12 less efficiently. This can slightly increase your dietary need for active B12."
  },
  "rs1801198": {
    simpleImpact: "Reduced Vitamin B12 delivery to cells (TCN2 gene)",
    simpleMeaning: "This variant makes the protein that carries vitamin B12 to your cells less efficient. Your cells might not get enough B12 even if blood tests show normal levels."
  },
  "rs602662": {
    simpleImpact: "Altered gut environment and lower blood B12 (FUT2 gene)",
    simpleMeaning: "Having two copies of this variant makes you a 'non-secretor,' meaning you don't release blood group markers into bodily fluids. This changes your gut bacteria and is linked to lower blood B12 levels."
  },
  "rs7946": {
    simpleImpact: "Reduced choline production in the liver (PEMT gene)",
    simpleMeaning: "This variant reduces your liver's ability to produce choline, an essential nutrient. This makes you much more sensitive to low-choline diets and increases the risk of fatty liver."
  },
  "rs2236225": {
    simpleImpact: "Altered folate processing and higher choline demand (MTHFD1 gene)",
    simpleMeaning: "This variant reduces your ability to process folate. This forces your body to rely more on choline, increasing your daily dietary choline requirement."
  },
  "rs2228570": {
    simpleImpact: "Slightly less active Vitamin D receptors (VDR gene)",
    simpleMeaning: "This variant makes your cells slightly less sensitive to Vitamin D. You might need higher blood levels of Vitamin D to get the same health benefits."
  },
  "rs1544410": {
    simpleImpact: "Fewer Vitamin D receptors (VDR gene)",
    simpleMeaning: "This variant is linked to having fewer Vitamin D receptors on your cells, which can reduce how effectively your body uses Vitamin D."
  },
  "rs2282679": {
    simpleImpact: "Lower Vitamin D transport in the blood (GC gene)",
    simpleMeaning: "This variant lowers the amount of transporter protein available to carry Vitamin D in your blood, which often leads to lower overall Vitamin D blood levels."
  },
  "rs10741657": {
    simpleImpact: "Less efficient Vitamin D activation in the liver (CYP2R1 gene)",
    simpleMeaning: "This variant slows down how your liver converts Vitamin D from sunlight or supplements into its active form, making it harder to maintain optimal levels."
  },
  "rs12785878": {
    simpleImpact: "Reduced Vitamin D production from sunlight (DHCR7 gene)",
    simpleMeaning: "This variant makes your skin less efficient at producing Vitamin D when exposed to sunlight, increasing your reliance on dietary sources and supplements."
  },
  "rs3892097": {
    simpleImpact: "CYP2D6*4 no-function allele component — clinical PGx needed",
    simpleMeaning: "This is one no-function CYP2D6 allele component. A clinical CYP2D6 diplotype and copy-number result are needed before discussing a medicine; do not change medication from this SNP alone."
  },
  "rs1057910": {
    simpleImpact: "CYP2C9*3 reduced-function allele component — clinical PGx needed",
    simpleMeaning: "This is one reduced-function CYP2C9 allele component. A clinical diplotype, the specific drug, and prescribing context are needed before estimating medication effects."
  },
  "rs1799853": {
    simpleImpact: "CYP2C9*2 reduced-function allele component — clinical PGx needed",
    simpleMeaning: "This is one reduced-function CYP2C9 allele component. It is not a complete medication-response result; use validated clinical PGx interpretation and a clinician or pharmacist."
  },
  "rs12248560": {
    simpleImpact: "CYP2C19*17 increased-function allele component — clinical PGx needed",
    simpleMeaning: "This is one CYP2C19 increased-function allele component. A full diplotype and the specific medication are needed; do not assume ultra-rapid metabolism from this SNP alone."
  },
  "rs4244285": {
    simpleImpact: "CYP2C19*2 loss-of-function allele component — clinical PGx needed",
    simpleMeaning: "This is one CYP2C19 loss-of-function allele component. A clinical diplotype and drug-specific guideline are needed before discussing clopidogrel or another medication; this SNP alone is not a phenotype."
  },
  "rs4986893": {
    simpleImpact: "CYP2C19*3 loss-of-function allele component — clinical PGx needed",
    simpleMeaning: "This is one CYP2C19 loss-of-function allele component. It should be combined with the other allele and the drug-specific guideline in a validated clinical PGx result."
  },
  "rs4149056": {
    simpleImpact: "SLCO1B1 reduced-function allele component — statin context needs clinical review",
    simpleMeaning: "This is one SLCO1B1 transporter allele component studied in statin response. The statin, full haplotype, symptoms, and clinical guidance matter; do not start or change a statin from this SNP alone."
  },
  "rs762551": {
    simpleImpact: "Slower breakdown of caffeine (CYP1A2 gene)",
    simpleMeaning: "You break down caffeine slowly. It stays in your body longer, which can cause jitteriness, anxiety, or trouble sleeping if you drink coffee or energy drinks later in the day."
  },
  "rs9923231": {
    simpleImpact: "VKORC1 warfarin-sensitivity allele component — clinical dosing required",
    simpleMeaning: "This is one VKORC1 component used with CYP2C9, CYP4F2, clinical factors, and INR monitoring when warfarin is prescribed. It does not determine a dose by itself."
  },
  "rs2108622": {
    simpleImpact: "CYP4F2 warfarin-dose component — clinical dosing required",
    simpleMeaning: "This is one CYP4F2 component considered in warfarin dosing. The complete clinical PGx result, diet, other medicines, and INR monitoring matter; it does not set a dose by itself."
  },
  "rs776746": {
    simpleImpact: "CYP3A5*3 splice allele component — clinical PGx needed",
    simpleMeaning: "This is a CYP3A5 splice-variant component. A clinical genotype, ancestry-aware interpretation, and the specific medication are needed; it is not a broad result for all medicines."
  },
  "rs3745274": {
    simpleImpact: "CYP2B6*6 reduced-function allele component — clinical PGx needed",
    simpleMeaning: "This is one CYP2B6 reduced-function allele component. The complete diplotype, medication, and clinical guideline are needed before discussing exposure or dose."
  },
  "rs2231142": {
    simpleImpact: "Reduced transport of uric acid and statins (ABCG2 gene)",
    simpleMeaning: "This variant makes it harder for your body to move certain drugs and uric acid. This can cause cholesterol medications to build up in your body and increases your risk of gout."
  },
  "rs3918290": {
    simpleImpact: "DPYD*2A toxicity-risk allele component — urgent clinical confirmation if relevant",
    simpleMeaning: "If this allele is confirmed by a clinical DPYD test, it can be important before fluoropyrimidine chemotherapy. A raw consumer call is not enough to choose, avoid, or dose treatment; oncology and PGx guidance are required."
  },
  "rs55886062": {
    simpleImpact: "Rare DPYD toxicity-risk allele component — urgent clinical confirmation if relevant",
    simpleMeaning: "If this rare allele is confirmed clinically, it may affect fluoropyrimidine safety. Do not infer treatment from a raw array call; an oncology team must use validated DPYD testing and current guidance."
  },
  "rs67376798": {
    simpleImpact: "DPYD reduced-function allele component — clinical confirmation if relevant",
    simpleMeaning: "This is one DPYD allele component studied for fluoropyrimidine toxicity. Confirm it clinically and let the oncology team interpret the complete result before treatment decisions."
  },
  "rs1800460": {
    simpleImpact: "TPMT reduced-function allele component — thiopurine safety requires clinical testing",
    simpleMeaning: "This is one TPMT allele component. A clinical TPMT/NUDT15 result, blood-count monitoring, and the prescribing guideline are needed before interpreting thiopurine risk."
  },
  "rs1142345": {
    simpleImpact: "TPMT*3C allele component — thiopurine safety requires clinical testing",
    simpleMeaning: "This is one TPMT allele component and can contribute to a TPMT diplotype. Confirm clinically with TPMT/NUDT15 guidance; do not infer a dose or stop treatment from this SNP alone."
  },
  "rs116855232": {
    simpleImpact: "NUDT15 reduced-function allele component — thiopurine safety requires clinical testing",
    simpleMeaning: "This is one NUDT15 allele component studied for thiopurine toxicity. Confirm with a clinical PGx result and blood-count monitoring before any treatment decision."
  },
  "rs73598374": {
    simpleImpact: "Higher sleep pressure and deeper sleep (ADA gene)",
    simpleMeaning: "This variant slows down how your body breaks down adenosine, a chemical that builds up during the day to make you feel sleepy. You may experience deeper sleep but feel groggier when you wake up."
  },
  "rs934945": {
    simpleImpact: "Sleep schedule disruption and lighter sleep (PER2 gene)",
    simpleMeaning: "This research-level variant in a core clock gene is linked to disruptions in your sleep schedule, waking up in the middle of the night, and lighter sleep."
  },
  "rs1801260": {
    simpleImpact: "Natural preference for staying up late (CLOCK gene)",
    simpleMeaning: "This variant is strongly linked to being a 'night owl.' It can delay your body's natural release of melatonin, making it harder to fall asleep early."
  },
  "rs10830963": {
    simpleImpact: "Altered sugar control in the morning (MTNR1B gene)",
    simpleMeaning: "This variant affects how your body handles sugar in the morning. When melatonin levels are still high, your body release less insulin, which can lead to higher morning blood sugar levels."
  },
  "rs225014": {
    simpleImpact: "Reduced thyroid hormone activation in cells (DIO2 gene)",
    simpleMeaning: "This variant makes your cells less efficient at turning inactive thyroid hormone into its active form. This can cause mild fatigue or brain fog even if your standard blood tests look normal."
  },
  "rs1050450": {
    simpleImpact: "Lower antioxidant protection in the thyroid (GPX1 gene)",
    simpleMeaning: "This variant reduces your body's ability to protect the thyroid gland from oxidative stress. Getting enough selenium in your diet can help support this protective pathway."
  },
  "rs231775": {
    simpleImpact: "Higher immune cell activation and autoimmune risk (CTLA4 gene)",
    simpleMeaning: "This variant lowers a safety brake on your immune cells, making them more active. This can slightly raise your risk for autoimmune conditions, where the body attacks its own tissues."
  },
  "rs2476601": {
    simpleImpact: "Altered immune signaling and autoimmune risk (PTPN22 gene)",
    simpleMeaning: "This variant changes a key signal in your immune cells, making it harder for your body to turn off immune responses. This is linked to a higher risk of autoimmune thyroid issues and other conditions."
  },
  "rs1883832": {
    simpleImpact: "Increased thyroid antibody production (CD40 gene)",
    simpleMeaning: "This variant increases the activity of immune cells that make antibodies. In some people, this can lead to the immune system targeting the thyroid gland."
  },
  "rs179247": {
    simpleImpact: "Altered thyroid hormone receptor sensitivity (TSHR gene)",
    simpleMeaning: "This variant affects the sensitivity of your thyroid receptors to hormone signals, which can influence your risk of autoimmune thyroid conditions."
  },
  "rs2235544": {
    simpleImpact: "Altered thyroid hormone conversion in organs (DIO1 gene)",
    simpleMeaning: "This variant alters how your liver and kidneys convert thyroid hormone from its inactive to active form, causing minor shifts in thyroid hormone ratios in the blood."
  },
};
