// ./src/lib/utils/layperson.ts
/**
 * Purpose: Provide clear, plain-English translations for genetic marker impacts and meanings.
 * Responsibilities:
 * - Define the LaypersonTranslation interface.
 * - Map curated rsIDs from the marker packs to their simplified layperson-friendly translations.
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
    simpleImpact: "APOE e4-associated marker (lipid and brain-health context)",
    simpleMeaning: "This marker is part of the APOE type used in lipid and age-related health research. Associations with cholesterol and other outcomes vary by the complete APOE type, ancestry, age, and health history; a lipid profile and clinical context are more useful than this SNP alone."
  },
  "rs7412": {
    simpleImpact: "APOE haplotype component",
    simpleMeaning: "This is one of the markers used with rs429358 to infer a common APOE type. This SNP alone does not establish the full type or your current cholesterol; use a lipid profile and clinical context."
  },
  "rs10757278": {
    simpleImpact: "9p21 cardiovascular-association marker",
    simpleMeaning: "This region marker has been associated with coronary-disease outcomes in some studies. The effect is probabilistic and overlaps with blood pressure, lipids, smoking, diabetes, family history, and other factors; it is not an early-heart-attack prediction."
  },
  "rs1799983": {
    simpleImpact: "NOS3 nitric-oxide signaling marker",
    simpleMeaning: "This NOS3 marker has been studied in nitric-oxide signaling and vascular response. Associations vary between studies; it does not measure nitric oxide, blood flow, or exercise recovery."
  },
  "rs2070744": {
    simpleImpact: "NOS3 vascular-response marker",
    simpleMeaning: "This NOS3 marker has been studied in relation to nitric-oxide signaling and vascular response. It is a small-effect research association, not a measurement of vessel relaxation or cardiovascular disease."
  },
  "rs1800012": {
    simpleImpact: "COL1A1 bone and collagen association marker",
    simpleMeaning: "This COL1A1 marker has been associated with small differences in bone or connective-tissue traits. Effects are probabilistic and do not establish bone weakness or injury risk; symptoms, fracture history, and clinical testing matter more."
  },
  "rs12722": {
    simpleImpact: "Tendon and ligament flexibility (COL5A1 gene)",
    simpleMeaning: "This COL5A1 marker has been studied in tendon, ligament, and flexibility traits. Associations vary; it does not predict an injury or establish whether a person is unusually flexible."
  },
  "rs1800255": {
    simpleImpact: "Common collagen structure variant (COL3A1 gene)",
    simpleMeaning: "This common marker is not a diagnosis of a collagen disorder. It has been studied in relation to small differences in joint and tissue flexibility, but symptoms and clinical examination matter more."
  },
  "rs143383": {
    simpleImpact: "GDF5 joint-trait association marker",
    simpleMeaning: "This GDF5 marker has been studied in cartilage and osteoarthritis susceptibility. Any association is small and probabilistic; it does not diagnose osteoarthritis or predict which joint becomes symptomatic."
  },
  "rs3025058": {
    simpleImpact: "MMP3 tissue-remodeling association marker",
    simpleMeaning: "This MMP3 marker has been studied in tissue-remodeling traits in joints and tendons. Associations vary and may be small; it does not determine tendon integrity or predict an Achilles injury."
  },
  "rs2118181": {
    simpleImpact: "FBN1/connective-tissue association marker",
    simpleMeaning: "This common marker has been studied in connective-tissue and vascular traits. It does not establish vessel fragility, aortic disease, or a condition such as Marfan syndrome; clinical examination and family history are more informative."
  },
  "rs8033037": {
    simpleImpact: "Common bone growth variant (FBN1 gene)",
    simpleMeaning: "This common variant is linked to natural variations in height and bone length. It is not harmful and represents normal human diversity."
  },
  "rs1815739": {
    simpleImpact: "ACTN3 fast-fiber performance association marker",
    simpleMeaning: "Two copies are associated with absence of an ACTN3 protein in fast fibers in relevant research. Training, health, and many other genes influence performance, so this does not determine an endurance or sprint profile."
  },
  "rs970547": {
    simpleImpact: "COL12A1 ligament-trait association marker",
    simpleMeaning: "This COL12A1 marker has been studied in ligament and injury-related traits. Some studies report associations with ligament injury, but effects vary and it does not predict an ACL tear."
  },
  "rs3918242": {
    simpleImpact: "MMP9 tissue-remodeling association marker",
    simpleMeaning: "This MMP9 marker has been studied in tissue-remodeling and vascular or joint traits. Associations vary; it does not measure tissue function or establish joint or heart disease."
  },
  "rs10156191": {
    simpleImpact: "AOC1 histamine-processing research marker",
    simpleMeaning: "This AOC1 marker has been studied in DAO-related histamine processing. Food tolerance varies widely, and this SNP does not diagnose histamine intolerance, food allergy, hives, or a need to avoid particular foods."
  },
  "rs11558538": {
    simpleImpact: "HNMT histamine-processing research marker",
    simpleMeaning: "This HNMT marker has been studied in cellular histamine processing. Symptoms and food responses are not determined by this SNP; consider allergies, medicines, other conditions, and clinical evaluation when symptoms persist."
  },
  "rs4988235": {
    simpleImpact: "MCM6 lactase-persistence association marker",
    simpleMeaning: "Two copies are associated with adult lactase non-persistence in many European-ancestry studies. Symptoms and dairy tolerance vary, and this is not a milk-allergy diagnosis."
  },
  "rs9939609": {
    simpleImpact: "FTO appetite and weight association marker",
    simpleMeaning: "This FTO marker has been associated with small differences in appetite or weight-related traits in some populations. It does not determine appetite or body size; eating patterns, activity, sleep, environment, and health history matter."
  },
  "rs1801282": {
    simpleImpact: "PPARG metabolic association marker",
    simpleMeaning: "This PPARG marker is studied in fat storage and insulin sensitivity. Associations are modest and context-dependent; it does not establish insulin sensitivity or protection from type 2 diabetes."
  },
  "rs2237897": {
    simpleImpact: "KCNQ1 glucose and insulin-secretion association marker",
    simpleMeaning: "This KCNQ1 marker has been associated with small differences in insulin secretion or glucose-related traits. It does not diagnose diabetes or predict an individual response; HbA1c, glucose testing, and clinical risk factors are more useful."
  },
  "rs7903146": {
    simpleImpact: "TCF7L2 glucose association marker",
    simpleMeaning: "This is a well-studied common association with type 2 diabetes risk, but it is not a diagnosis or a personal probability estimate. Glucose, HbA1c, family history, age, body composition, activity, sleep, and overall diet provide the actionable context."
  },
  "rs13266634": {
    simpleImpact: "SLC30A8 beta-cell association marker",
    simpleMeaning: "This SLC30A8 marker is studied in zinc transport and pancreatic beta-cell function. Any association with glucose or diabetes risk is probabilistic; it does not measure insulin release or diagnose diabetes."
  },
  "rs5219": {
    simpleImpact: "KCNJ11 beta-cell association marker",
    simpleMeaning: "This KCNJ11 marker has been studied in pancreatic beta-cell function and glucose traits. It does not measure post-meal insulin or establish diabetes risk for one person; use laboratory testing and clinical context."
  },
  "rs1260326": {
    simpleImpact: "GCKR glucose and triglyceride association marker",
    simpleMeaning: "This GCKR marker has been associated with a trade-off between fasting glucose and triglyceride-related traits in some studies. A lipid panel and glucose testing show current status; the SNP is not a diagnosis."
  },
  "rs662799": {
    simpleImpact: "APOA5 triglyceride association marker",
    simpleMeaning: "This APOA5 marker has been associated with triglyceride levels in some populations. Diet, alcohol, metabolic health, medicines, and other genes also matter; a measured lipid panel is needed."
  },
  "rs328": {
    simpleImpact: "LPL lipid-clearance association marker",
    simpleMeaning: "This variant has been associated with differences in an enzyme involved in blood-fat clearance. Some studies link it with lower triglycerides or higher HDL, but your actual lipid tests and overall health matter more."
  },
  "rs708272": {
    simpleImpact: "Increased good (HDL) cholesterol (CETP gene)",
    simpleMeaning: "This marker has been associated with differences in cholesterol transport and HDL levels. It does not guarantee protection; a complete lipid profile and ApoB-related context are more useful."
  },
  "rs10455872": {
    simpleImpact: "LPA lipoprotein(a)-level association marker",
    simpleMeaning: "This LPA marker has been associated with higher lipoprotein(a) in many studies, but the SNP does not measure your level. A measured Lp(a) result, lipid profile, family history, and overall cardiovascular risk are needed for useful interpretation."
  },
  "rs3798220": {
    simpleImpact: "Elevated Lipoprotein(a) levels (LPA gene)",
    simpleMeaning: "This LPA marker is associated with a smaller lipoprotein(a) form and may be associated with higher Lp(a) in some people. A measured Lp(a) test is needed; the SNP alone is not a cardiovascular diagnosis."
  },
  "rs17782313": {
    simpleImpact: "Altered fullness signaling (MC4R gene)",
    simpleMeaning: "This MC4R-region marker has been associated with small differences in appetite or weight-related traits. It does not determine hunger or eating behavior; food environment, sleep, activity, and health history matter."
  },
  "rs1360780": {
    simpleImpact: "Altered stress hormone sensitivity (FKBP5 gene)",
    simpleMeaning: "This FKBP5 marker has been studied in stress-response regulation. Associations vary by stress exposure and other factors; it does not measure cortisol or determine emotional responses."
  },
  "rs6311": {
    simpleImpact: "Altered serotonin receptor density (HTR2A gene)",
    simpleMeaning: "This research-level HTR2A marker has been studied in serotonin-related traits and medication-response research. It does not measure receptor density, diagnose a mood condition, or predict a medication response."
  },
  "rs53576": {
    simpleImpact: "Altered oxytocin (social hormone) sensitivity (OXTR gene)",
    simpleMeaning: "This OXTR marker has been studied in oxytocin-related and social-behavior research. Findings are inconsistent and context-dependent; it does not determine bonding, empathy, personality, or how someone handles stress."
  },
  "rs6323": {
    simpleImpact: "Slower breakdown of brain chemicals (MAOA gene)",
    simpleMeaning: "This MAOA marker has been studied in neurotransmitter metabolism and behavior research. It does not measure brain-chemical levels or determine emotional responses, aggression, or personality."
  },
  "rs4680": {
    simpleImpact: "Slower dopamine breakdown in the brain (COMT gene)",
    simpleMeaning: "This is one COMT allele component studied in dopamine and catechol-estrogen breakdown. Effects are small and context-dependent; it does not determine focus, anxiety, personality, or hormone levels."
  },
  "rs1800497": {
    simpleImpact: "Fewer dopamine receptors (DRD2/ANKK1 gene region)",
    simpleMeaning: "This DRD2/ANKK1-region marker has been studied in dopamine signaling and reward-related traits. It does not measure receptor number or determine motivation, addiction, or reward-seeking behavior."
  },
  "rs6277": {
    simpleImpact: "Altered dopamine receptor availability (DRD2 gene)",
    simpleMeaning: "This DRD2 marker has been studied in dopamine signaling and cognitive traits. Associations are small and inconsistent; it does not measure receptor availability or determine memory, learning, or flexibility."
  },
  "rs7794154": {
    simpleImpact: "CNTNAP2 brain-connectivity research marker",
    simpleMeaning: "This research-level CNTNAP2 marker has been studied in brain connectivity, language development, and sensory traits. It does not establish a neurodevelopmental condition or predict an individual's abilities."
  },
  "rs4307059": {
    simpleImpact: "MSNP1AS brain-structure research marker",
    simpleMeaning: "This minor research marker has been studied in brain structure, sensory processing, and neurodevelopmental traits. The association is weak and does not predict an individual's traits or diagnosis."
  },
  "rs1800544": {
    simpleImpact: "ADRA2A norepinephrine-signaling association marker",
    simpleMeaning: "This ADRA2A marker has been studied in norepinephrine signaling and attention-related research. It does not measure receptor sensitivity or predict focus, stress response, or medication outcome."
  },
  "rs1801132": {
    simpleImpact: "ESR1 hormone-response research marker",
    simpleMeaning: "This ESR1 marker has been studied in estrogen-receptor and hormone-response research. It does not measure current estrogen, determine hormone sensitivity, or explain menstrual mood symptoms."
  },
  "rs2234693": {
    simpleImpact: "ESR1 hormone-response research marker",
    simpleMeaning: "This research-level ESR1 marker has been studied in estrogen-receptor and symptom-timing research. It does not measure estrogen or explain emotional or physical symptoms during a hormone change."
  },
  "rs8079626": {
    simpleImpact: "PGR hormone-response research marker",
    simpleMeaning: "This PGR marker has been studied in progesterone-receptor and menstrual-symptom research. It does not measure progesterone, determine receptor sensitivity, or diagnose a cycle-related mood condition."
  },
  "rs6265": {
    simpleImpact: "BDNF neuroplasticity association marker",
    simpleMeaning: "This BDNF marker has been studied in neuroplasticity, memory, and exercise-response research. Associations vary; it does not measure BDNF, predict learning ability, or establish exercise response."
  },
  "rs324420": {
    simpleImpact: "FAAH endocannabinoid-signaling association marker",
    simpleMeaning: "This FAAH marker has been studied in endocannabinoid signaling and stress-related traits. It does not measure anandamide, diagnose anxiety, or predict stress resilience."
  },
  "rs5751876": {
    simpleImpact: "ADORA2A caffeine-sensitivity association marker",
    simpleMeaning: "This ADORA2A marker has been associated with differences in caffeine sensitivity in some studies. Dose, sleep, expectations, medicines, pregnancy, and personal response matter; it does not diagnose anxiety or set a universal caffeine limit."
  },
  "rs1799971": {
    simpleImpact: "OPRM1 pain and reward association marker",
    simpleMeaning: "This OPRM1 marker has been studied in endorphin signaling, pain, and reward-related traits. Associations are small or inconsistent; it does not determine pain tolerance, social bonding, or medication response."
  },
  "rs1611115": {
    simpleImpact: "DBH dopamine-to-norepinephrine association marker",
    simpleMeaning: "This DBH marker has been studied in dopamine-to-norepinephrine metabolism. It does not measure neurotransmitter levels or determine focus, stress response, or medication outcome."
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
    simpleImpact: "IL6 inflammation-association marker",
    simpleMeaning: "This IL6 marker has been studied in inflammatory signaling and measured IL-6 differences. It does not measure current inflammation or predict how someone feels during illness or stress."
  },
  "rs1800629": {
    simpleImpact: "TNF inflammation-association marker",
    simpleMeaning: "This TNF marker has been studied in inflammatory signaling. Associations vary by population and context; it does not measure current TNF levels, diagnose inflammation, or establish autoimmune disease."
  },
  "rs1800562": {
    simpleImpact: "HFE iron-overload evaluation marker",
    simpleMeaning: "This HFE variant can be relevant to iron-overload evaluation, especially when a clinical result and iron studies support it. A raw SNP result alone does not diagnose hemochromatosis or prove that you absorb too much iron."
  },
  "rs1799945": {
    simpleImpact: "HFE iron-related association marker",
    simpleMeaning: "This is a milder HFE component. Its significance depends on other HFE alleles, iron studies, symptoms, and life stage; it is not proof of iron buildup by itself."
  },
  "rs1805087": {
    simpleImpact: "Altered folate and B12 processing (MTR gene)",
    simpleMeaning: "This MTR marker is studied in folate and vitamin-B12 pathways. Any effect is likely small and context-dependent; it does not establish a vitamin deficiency or determine supplement need."
  },
  "rs1801394": {
    simpleImpact: "MTRR vitamin-B12 pathway association marker",
    simpleMeaning: "This MTRR marker is studied in vitamin-B12 recycling. It does not measure B12 status or set a personal dietary requirement; symptoms, diet, absorption, medicines, and laboratory results matter."
  },
  "rs1801198": {
    simpleImpact: "Reduced Vitamin B12 delivery to cells (TCN2 gene)",
    simpleMeaning: "This variant has been studied in vitamin-B12 transport. If symptoms or labs raise concern, clinicians may consider more than a single serum B12 result; the SNP alone does not establish cellular B12 deficiency."
  },
  "rs602662": {
    simpleImpact: "FUT2 secretor-status and B12 association marker",
    simpleMeaning: "This FUT2 marker has been associated with secretor-status and B12 differences in some populations. One SNP does not establish secretor status or B12 deficiency; symptoms, diet, and labs provide the useful context."
  },
  "rs7946": {
    simpleImpact: "PEMT choline-pathway association marker",
    simpleMeaning: "This PEMT marker is studied in choline metabolism. It may be a reason to consider ordinary choline-rich foods in the context of your diet and health, but it does not establish a choline deficiency or fatty-liver risk."
  },
  "rs2236225": {
    simpleImpact: "MTHFD1 folate/choline-pathway association marker",
    simpleMeaning: "This MTHFD1 marker is studied in folate and choline pathways. Any effect is likely context-dependent; it does not set a personal daily choline requirement or justify high-dose supplements."
  },
  "rs2228570": {
    simpleImpact: "Slightly less active Vitamin D receptors (VDR gene)",
    simpleMeaning: "This VDR marker is associated in some studies with small differences in vitamin-D signaling. Use 25(OH)D and clinical context rather than the SNP to decide whether any action is needed."
  },
  "rs1544410": {
    simpleImpact: "VDR vitamin-D signaling association marker",
    simpleMeaning: "This VDR marker has been studied in vitamin-D signaling. It does not measure receptor number or current vitamin-D status; use a 25(OH)D result and clinical context rather than the SNP alone."
  },
  "rs2282679": {
    simpleImpact: "Lower Vitamin D transport in the blood (GC gene)",
    simpleMeaning: "This GC marker has been associated with differences in vitamin-D transport or measured levels in some populations. A blood test, not the SNP alone, shows your current vitamin-D status."
  },
  "rs10741657": {
    simpleImpact: "Less efficient Vitamin D activation in the liver (CYP2R1 gene)",
    simpleMeaning: "This CYP2R1 marker is studied in vitamin-D activation. Effects vary; use measured 25(OH)D and clinician guidance rather than assuming supplementation is needed."
  },
  "rs12785878": {
    simpleImpact: "DHCR7 vitamin-D association marker",
    simpleMeaning: "This DHCR7 marker is associated in some studies with vitamin-D-related differences. It does not measure sunlight exposure or establish reliance on supplements; use labs and safe sun/food guidance."
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
    simpleMeaning: "This CYP1A2 marker is one contributor to caffeine-response research. Your response also depends on dose, sleep, smoking, medicines, pregnancy, and other factors; use your symptoms and blood pressure rather than a DNA-only limit."
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
    simpleMeaning: "This ABCG2 marker is studied in uric-acid handling and transport of some medicines. It may modify gout or medication context, but it is not a gout diagnosis or a medication-dose result."
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
    simpleImpact: "ADA sleep-pressure association marker",
    simpleMeaning: "This ADA marker has been studied in adenosine metabolism and sleep-pressure traits. It does not predict sleep depth or morning alertness; sleep schedule, health, medicines, and environment matter."
  },
  "rs934945": {
    simpleImpact: "PER2 circadian-rhythm research marker",
    simpleMeaning: "This research-level PER2 marker has been studied in circadian timing and sleep traits. It does not diagnose insomnia or predict awakenings; sleep history and clinical context matter."
  },
  "rs1801260": {
    simpleImpact: "CLOCK circadian-timing association marker",
    simpleMeaning: "This CLOCK marker has been associated with chronotype in some studies. It does not determine whether someone is a night owl or measure melatonin; light exposure, schedule, sleep pressure, and environment also matter."
  },
  "rs10830963": {
    simpleImpact: "Altered sugar control in the morning (MTNR1B gene)",
    simpleMeaning: "This MTNR1B marker has been associated with fasting-glucose and glucose-timing traits. It does not measure melatonin or insulin release; sleep timing, meals, medicines, and glucose testing provide the useful context."
  },
  "rs225014": {
    simpleImpact: "DIO2 thyroid-hormone-conversion association marker",
    simpleMeaning: "This DIO2 marker has been studied in thyroid-hormone conversion. It does not measure tissue thyroid activity or explain fatigue/brain fog; thyroid symptoms and laboratory results require clinical interpretation."
  },
  "rs1050450": {
    simpleImpact: "GPX1 antioxidant-pathway association marker",
    simpleMeaning: "This GPX1 marker has been studied in antioxidant-enzyme activity and thyroid-related research. It does not measure selenium status or establish a need for selenium; food, laboratory results, medicines, and clinician guidance matter."
  },
  "rs231775": {
    simpleImpact: "CTLA4 immune-association marker",
    simpleMeaning: "This CTLA4 marker has been associated with autoimmune traits in some studies. It does not measure immune activity or diagnose an autoimmune condition; symptoms, examination, and appropriate tests are needed."
  },
  "rs2476601": {
    simpleImpact: "Altered immune signaling and autoimmune risk (PTPN22 gene)",
    simpleMeaning: "This PTPN22 marker has been associated with several autoimmune traits. Associations vary by condition and population; it does not diagnose autoimmune thyroid disease or predict an individual outcome."
  },
  "rs1883832": {
    simpleImpact: "CD40 immune-signaling association marker",
    simpleMeaning: "This CD40 marker has been studied in antibody and autoimmune-trait research. It does not measure thyroid antibodies or diagnose thyroid disease; symptoms and laboratory testing are required."
  },
  "rs179247": {
    simpleImpact: "TSHR thyroid and autoimmune-association marker",
    simpleMeaning: "This TSHR marker has been studied in thyroid signaling and autoimmune-thyroid traits. Associations vary by condition and population; it does not measure receptor sensitivity or diagnose thyroid disease."
  },
  "rs2235544": {
    simpleImpact: "Altered thyroid hormone conversion in organs (DIO1 gene)",
    simpleMeaning: "This DIO1 marker has been studied in thyroid-hormone conversion and measured hormone-ratio differences. It does not measure current thyroid function or determine treatment; laboratory results and clinical assessment matter."
  },
};
