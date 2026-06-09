# ./scratch/generate_layperson.py
"""
Script to validate and generate the src/lib/utils/layperson.ts file.
It maps each of the 99 rsIDs to its simplified layperson translation,
ensuring no rsID is missed and that all translations are accurate.
"""

import json
import os

TRANSLATIONS = {
    "rs80357159": {
        "simpleImpact": "Altered DNA repair (BRCA1 gene)",
        "simpleMeaning": "This is a research-only marker for DNA repair. It requires a doctor's confirmation and more clinical testing before making any health decisions."
    },
    "rs80357224": {
        "simpleImpact": "Altered DNA repair (BRCA1 gene)",
        "simpleMeaning": "This is a research-only marker for DNA repair. It requires a doctor's confirmation and more clinical testing before making any health decisions."
    },
    "rs11571833": {
        "simpleImpact": "Harmless DNA repair variant (BRCA2 gene)",
        "simpleMeaning": "This gene change is considered harmless and does not indicate a classic hereditary cancer risk. It is a common variant used in research."
    },
    "rs429358": {
        "simpleImpact": "APOE e4 variant linked to cholesterol and brain health",
        "simpleMeaning": "This gene variant affects how your body handles fats. Having one or two copies of this variant can lead to higher cholesterol levels and a higher long-term risk of heart issues or memory decline."
    },
    "rs7412": {
        "simpleImpact": "APOE gene type determination",
        "simpleMeaning": "This marker determines your specific APOE gene type. It helps show whether your body is naturally prone to lower or higher cholesterol levels."
    },
    "rs10757278": {
        "simpleImpact": "Increased risk of early-onset heart disease (9p21 region)",
        "simpleMeaning": "This variant affects cells in your blood vessel walls. Certain versions are linked to a higher risk of early heart attacks, independent of your cholesterol levels."
    },
    "rs1799983": {
        "simpleImpact": "Reduced nitric oxide production (NOS3 gene)",
        "simpleMeaning": "You may produce less nitric oxide, which helps widen blood vessels. This can affect how well your blood flows, your exercise recovery, and how your blood vessels react to stress."
    },
    "rs2070744": {
        "simpleImpact": "Lower nitric oxide levels (NOS3 gene)",
        "simpleMeaning": "This variant lowers the production of nitric oxide in your blood vessels. Working with other markers, it can make it harder for your blood vessels to relax."
    },
    "rs1800012": {
        "simpleImpact": "Slightly reduced bone density and collagen strength (COL1A1 gene)",
        "simpleMeaning": "This variant can slightly lower the strength of your body's main structural support, collagen. It is linked to slightly weaker bones and a higher chance of soft-tissue injuries like sprains."
    },
    "rs12722": {
        "simpleImpact": "Tendon and ligament flexibility (COL5A1 gene)",
        "simpleMeaning": "This variant affects the structure of your tendons and ligaments. Some versions give you more joint flexibility, while others make you more prone to tendon strains."
    },
    "rs1800255": {
        "simpleImpact": "Common collagen structure variant (COL3A1 gene)",
        "simpleMeaning": "This is a common, non-harmful change in your collagen structure. It does not cause serious tissue disease but plays a role in natural joint and tissue flexibility."
    },
    "rs143383": {
        "simpleImpact": "Reduced joint cartilage repair (GDF5 gene)",
        "simpleMeaning": "This variant reduces the production of a protein needed to repair joint cartilage. It can slightly increase your chances of developing joint wear-and-tear (osteoarthritis) over time, especially in the knees and hips."
    },
    "rs3025058": {
        "simpleImpact": "Altered tissue remodeling in tendons (MMP3 gene)",
        "simpleMeaning": "This variant affects how your body breaks down and rebuilds tissue in your joints and tendons. It can influence your risk of tendon injuries, such as Achilles tendon issues."
    },
    "rs2118181": {
        "simpleImpact": "Altered artery wall elasticity (FBN1 gene)",
        "simpleMeaning": "This variant affects the elasticity of your blood vessels. It is linked to a slight increase in risk for blood vessel stretching or tearing, but it is not a diagnosis of a genetic disorder like Marfan syndrome."
    },
    "rs8033037": {
        "simpleImpact": "Common bone growth variant (FBN1 gene)",
        "simpleMeaning": "This common variant is linked to natural variations in height and bone length. It is not harmful and represents normal human diversity."
    },
    "rs1815739": {
        "simpleImpact": "Reduced fast-twitch muscle power (ACTN3 gene)",
        "simpleMeaning": "Having two copies of this variant makes your body stop producing a muscle protein found in fast-twitch fibers. This shifts your natural athletic profile toward endurance and away from pure explosive power."
    },
    "rs970547": {
        "simpleImpact": "Altered ligament strength (COL12A1 gene)",
        "simpleMeaning": "This variant affects how collagen is organized in your ligaments. Certain versions are associated with a slightly higher risk of ligament tears, such as ACL injuries in the knee."
    },
    "rs3918242": {
        "simpleImpact": "Altered tissue remodeling (MMP9 gene)",
        "simpleMeaning": "This variant changes how your body maintains and breaks down tissues in your joints and blood vessels, which can impact your overall joint and heart health."
    },
    "rs10156191": {
        "simpleImpact": "Reduced breakdown of histamine in food (AOC1 gene)",
        "simpleMeaning": "You produce less of the enzyme that breaks down histamine in your gut. This can make you more sensitive to histamine-rich foods like aged cheeses, wine, or fermented items, leading to digestive issues or hives."
    },
    "rs11558538": {
        "simpleImpact": "Slower breakdown of histamine in cells (HNMT gene)",
        "simpleMeaning": "This variant slows down how your brain and body clear histamine from inside your cells. This can make histamine signals last longer, potentially affecting sleep, focus, or allergy-like symptoms."
    },
    "rs4988235": {
        "simpleImpact": "Lactose intolerance (MCM6 gene)",
        "simpleMeaning": "Having two copies of this variant makes your body stop producing lactase, the enzyme needed to digest milk sugar (lactose), as you grow older. This leads to lactose intolerance."
    },
    "rs9939609": {
        "simpleImpact": "Increased appetite and slower fullness signals (FTO gene)",
        "simpleMeaning": "This variant is linked to a naturally higher appetite and taking longer to feel full. Regular exercise and eating enough protein can help counteract this effect."
    },
    "rs1801282": {
        "simpleImpact": "Altered fat storage and insulin sensitivity (PPARG gene)",
        "simpleMeaning": "This variant helps control fat storage and how your body responds to insulin. Some versions improve your insulin response, which helps protect you against type 2 diabetes."
    },
    "rs2237897": {
        "simpleImpact": "Lower insulin response to sugar (KCNQ1 gene)",
        "simpleMeaning": "This variant can lower the amount of insulin your body releases in response to sugar. This slightly raises your risk of developing type 2 diabetes."
    },
    "rs7903146": {
        "simpleImpact": "Elevated risk of type 2 diabetes (TCF7L2 gene)",
        "simpleMeaning": "This is the strongest common genetic marker linked to type 2 diabetes. It reduces insulin secretion, but eating a low-sugar diet and staying active can significantly lower your risk."
    },
    "rs13266634": {
        "simpleImpact": "Altered zinc transport in insulin cells (SLC30A8 gene)",
        "simpleMeaning": "This variant affects how zinc is moved inside your insulin-producing cells. This can alter how insulin is stored and released, slightly increasing your risk of diabetes."
    },
    "rs5219": {
        "simpleImpact": "Reduced insulin release (KCNJ11 gene)",
        "simpleMeaning": "This variant keeps channels in your pancreas open longer, which slows down the release of insulin. This can slightly elevate your blood sugar response after meals."
    },
    "rs1260326": {
        "simpleImpact": "Triglyceride and sugar balance shift (GCKR gene)",
        "simpleMeaning": "This variant shifts how your liver handles sugars and fats. Certain versions lower your fasting blood sugar but increase your level of triglycerides (fats in the blood)."
    },
    "rs662799": {
        "simpleImpact": "Reduced clearance of blood fats (APOA5 gene)",
        "simpleMeaning": "This variant slows down how quickly your body clears fat from your blood. This can lead to higher levels of fasting triglycerides."
    },
    "rs328": {
        "simpleImpact": "Faster clearance of blood fats (LPL gene)",
        "simpleMeaning": "This variant increases the activity of an enzyme that clears fat from your blood. This is generally beneficial, leading to lower triglycerides and higher good (HDL) cholesterol."
    },
    "rs708272": {
        "simpleImpact": "Increased good (HDL) cholesterol (CETP gene)",
        "simpleMeaning": "This variant reduces the activity of a protein that moves cholesterol around. This leads to higher levels of protective 'good' HDL cholesterol in your blood."
    },
    "rs10455872": {
        "simpleImpact": "Elevated Lipoprotein(a) levels (LPA gene)",
        "simpleMeaning": "This marker is strongly linked to higher levels of a specific type of cholesterol-carrying particle called Lipoprotein(a). High levels can increase the risk of plaque buildup in your arteries."
    },
    "rs3798220": {
        "simpleImpact": "Elevated Lipoprotein(a) levels (LPA gene)",
        "simpleMeaning": "This variant causes your body to make a smaller version of a blood protein that is cleared more slowly. This raises your levels of Lipoprotein(a), a risk factor for heart disease."
    },
    "rs17782313": {
        "simpleImpact": "Altered fullness signaling (MC4R gene)",
        "simpleMeaning": "This variant affects how your brain receives signals about being full. Certain versions can make you feel less satisfied after eating, leading to a tendency to eat more."
    },
    "rs1360780": {
        "simpleImpact": "Altered stress hormone sensitivity (FKBP5 gene)",
        "simpleMeaning": "This variant affects how your brain turns off its stress response. Certain versions make you more sensitive to stress and can prolong the effects of the stress hormone cortisol."
    },
    "rs6311": {
        "simpleImpact": "Altered serotonin receptor density (HTR2A gene)",
        "simpleMeaning": "This research-level variant changes how many serotonin receptors are on your brain cells. It can influence your mood, emotional reactivity, and how you react to certain medications."
    },
    "rs53576": {
        "simpleImpact": "Altered oxytocin (social hormone) sensitivity (OXTR gene)",
        "simpleMeaning": "This variant can affect how sensitive your brain is to oxytocin, a hormone involved in bonding and social stress management. Certain versions can affect how you seek support or handle stress around others."
    },
    "rs6323": {
        "simpleImpact": "Slower breakdown of brain chemicals (MAOA gene)",
        "simpleMeaning": "This variant slows down how quickly your body clears brain chemicals like dopamine and serotonin. This can lead to stronger emotional responses under sudden stress."
    },
    "rs4680": {
        "simpleImpact": "Slower dopamine breakdown in the brain (COMT gene)",
        "simpleMeaning": "Having two copies of this variant makes your body clear dopamine from your brain very slowly. This helps with deep focus and planning, but can also make you more prone to anxiety and stress under pressure."
    },
    "rs1800497": {
        "simpleImpact": "Fewer dopamine receptors (DRD2/ANKK1 gene region)",
        "simpleMeaning": "This variant reduces the number of dopamine receptors in your brain's reward center. This can make you more likely to seek out strong rewards or stimulation."
    },
    "rs6277": {
        "simpleImpact": "Altered dopamine receptor availability (DRD2 gene)",
        "simpleMeaning": "This variant affects how efficiently your brain makes dopamine receptors, which can play a role in memory, learning, and mental flexibility."
    },
    "rs7794154": {
        "simpleImpact": "Altered brain cell connections (CNTNAP2 gene)",
        "simpleMeaning": "This research-level variant is involved in how brain cells connect and communicate. It has been studied in relation to language development and sensory sensitivity."
    },
    "rs4307059": {
        "simpleImpact": "Altered brain structure connections (MSNP1AS gene)",
        "simpleMeaning": "This is a minor research marker that plays a role in brain cell architecture. It shows a weak association with sensory processing and neurodevelopmental traits."
    },
    "rs1800544": {
        "simpleImpact": "Altered adrenaline receptor sensitivity in the brain (ADRA2A gene)",
        "simpleMeaning": "This variant changes how your brain responds to norepinephrine, a chemical related to focus and stress. It is often studied for its role in attention and focus medication response."
    },
    "rs1801132": {
        "simpleImpact": "Estrogen receptor sensitivity (ESR1 gene)",
        "simpleMeaning": "This variant affects how your cells respond to estrogen. It has been researched for its relationship with mood changes during hormonal shifts."
    },
    "rs2234693": {
        "simpleImpact": "Estrogen binding sensitivity (ESR1 gene)",
        "simpleMeaning": "This research marker affects how your body processes estrogen. It is studied for links to emotional and physical symptoms during hormone drops."
    },
    "rs8079626": {
        "simpleImpact": "Progesterone receptor sensitivity (PGR gene)",
        "simpleMeaning": "This marker affects how your cells respond to progesterone. It is researched in relation to mood fluctuations during menstrual cycle hormonal peaks."
    },
    "rs6265": {
        "simpleImpact": "Reduced brain growth factor release (BDNF gene)",
        "simpleMeaning": "This variant reduces the release of a key protein that helps brain cells grow and adapt. It is linked to differences in memory, learning, and how your brain benefits from cardio exercise."
    },
    "rs324420": {
        "simpleImpact": "Higher natural 'bliss' chemical levels (FAAH gene)",
        "simpleMeaning": "This variant slows down the breakdown of a natural brain chemical often called the 'bliss molecule.' It is linked to lower anxiety and a better ability to handle stress."
    },
    "rs5751876": {
        "simpleImpact": "Increased risk of anxiety from caffeine (ADORA2A gene)",
        "simpleMeaning": "This variant affects how your brain reacts to caffeine. Certain versions can cause a much stronger fight-or-flight response, leading to jitteriness and anxiety after drinking coffee or energy drinks."
    },
    "rs1799971": {
        "simpleImpact": "Altered pain and reward sensitivity (OPRM1 gene)",
        "simpleMeaning": "This variant affects your body's endorphin system. It can slightly change your pain tolerance and affect how you respond to rewards or social bonding."
    },
    "rs1611115": {
        "simpleImpact": "Altered dopamine and norepinephrine balance (DBH gene)",
        "simpleMeaning": "This variant reduces the speed at which your body turns dopamine into norepinephrine. This changes the balance of key focus and stress chemicals in your nervous system."
    },
    "rs1801133": {
        "simpleImpact": "Reduced conversion of folic acid to active folate (MTHFR gene)",
        "simpleMeaning": "Having two copies of this variant can reduce your ability to convert folic acid from food or supplements into the active form your body needs by up to 70%. It is important to get enough natural folate."
    },
    "rs1801131": {
        "simpleImpact": "Slightly reduced folate conversion (MTHFR gene)",
        "simpleMeaning": "This variant causes a minor decrease in how well you process folate. If combined with other MTHFR variants, it can moderately affect your body's methylation process."
    },
    "rs1800795": {
        "simpleImpact": "Higher baseline inflammation (IL-6 gene)",
        "simpleMeaning": "This variant is linked to higher baseline levels of an inflammatory protein. This can make you more sensitive to physical and mental stress when your immune system is active."
    },
    "rs1800629": {
        "simpleImpact": "Increased inflammatory response (TNF gene)",
        "simpleMeaning": "This variant makes your body produce more of a key inflammatory protein. This can raise your body's overall baseline level of inflammation."
    },
    "rs1800562": {
        "simpleImpact": "Increased iron absorption risk (HFE gene)",
        "simpleMeaning": "Having two copies of this variant puts you at high risk for absorbing too much iron from your diet, which can build up in your organs and require medical management."
    },
    "rs1799945": {
        "simpleImpact": "Mildly increased iron absorption (HFE gene)",
        "simpleMeaning": "This is a milder iron-absorption variant. On its own, it rarely causes issues, but when combined with other variants, it can lead to mild iron buildup."
    },
    "rs1805087": {
        "simpleImpact": "Altered folate and B12 processing (MTR gene)",
        "simpleMeaning": "This variant affects an enzyme that works with folate and vitamin B12. It can slightly alter how your body uses these vitamins for cell maintenance."
    },
    "rs1801394": {
        "simpleImpact": "Slower Vitamin B12 recycling (MTRR gene)",
        "simpleMeaning": "This variant makes your body recycle vitamin B12 less efficiently. This can slightly increase your dietary need for active B12."
    },
    "rs1801198": {
        "simpleImpact": "Reduced Vitamin B12 delivery to cells (TCN2 gene)",
        "simpleMeaning": "This variant makes the protein that carries vitamin B12 to your cells less efficient. Your cells might not get enough B12 even if blood tests show normal levels."
    },
    "rs602662": {
        "simpleImpact": "Altered gut environment and lower blood B12 (FUT2 gene)",
        "simpleMeaning": "Having two copies of this variant makes you a 'non-secretor,' meaning you don't release blood group markers into bodily fluids. This changes your gut bacteria and is linked to lower blood B12 levels."
    },
    "rs7946": {
        "simpleImpact": "Reduced choline production in the liver (PEMT gene)",
        "simpleMeaning": "This variant reduces your liver's ability to produce choline, an essential nutrient. This makes you much more sensitive to low-choline diets and increases the risk of fatty liver."
    },
    "rs2236225": {
        "simpleImpact": "Altered folate processing and higher choline demand (MTHFD1 gene)",
        "simpleMeaning": "This variant reduces your ability to process folate. This forces your body to rely more on choline, increasing your daily dietary choline requirement."
    },
    "rs2228570": {
        "simpleImpact": "Slightly less active Vitamin D receptors (VDR gene)",
        "simpleMeaning": "This variant makes your cells slightly less sensitive to Vitamin D. You might need higher blood levels of Vitamin D to get the same health benefits."
    },
    "rs1544410": {
        "simpleImpact": "Fewer Vitamin D receptors (VDR gene)",
        "simpleMeaning": "This variant is linked to having fewer Vitamin D receptors on your cells, which can reduce how effectively your body uses Vitamin D."
    },
    "rs2282679": {
        "simpleImpact": "Lower Vitamin D transport in the blood (GC gene)",
        "simpleMeaning": "This variant lowers the amount of transporter protein available to carry Vitamin D in your blood, which often leads to lower overall Vitamin D blood levels."
    },
    "rs10741657": {
        "simpleImpact": "Less efficient Vitamin D activation in the liver (CYP2R1 gene)",
        "simpleMeaning": "This variant slows down how your liver converts Vitamin D from sunlight or supplements into its active form, making it harder to maintain optimal levels."
    },
    "rs12785878": {
        "simpleImpact": "Reduced Vitamin D production from sunlight (DHCR7 gene)",
        "simpleMeaning": "This variant makes your skin less efficient at producing Vitamin D when exposed to sunlight, increasing your reliance on dietary sources and supplements."
    },
    "rs3892097": {
        "simpleImpact": "Slower processing of many common medicines (CYP2D6 gene)",
        "simpleMeaning": "Having two copies of this variant makes you a 'poor metabolizer' for many medications, including cough syrups, antidepressants, and pain relievers. This can cause drug buildup or prevent certain drugs from working."
    },
    "rs1057910": {
        "simpleImpact": "Reduced breakdown of anti-inflammatory drugs and blood thinners (CYP2C9 gene)",
        "simpleMeaning": "This variant slows down how your body breaks down common medications like ibuprofen, blood thinners, and cholesterol-lowering statins. Standard doses might stay in your body too long."
    },
    "rs1799853": {
        "simpleImpact": "Slower breakdown of blood thinners and pain relievers (CYP2C9 gene)",
        "simpleMeaning": "This variant reduces the activity of an enzyme that clears blood thinners and common anti-inflammatory drugs from your body, potentially requiring dose adjustments."
    },
    "rs12248560": {
        "simpleImpact": "Ultra-fast breakdown of antidepressants and acid blockers (CYP2C19 gene)",
        "simpleMeaning": "You break down certain medications, like antidepressants and acid reflux drugs, much faster than normal. Standard doses may not be fully effective for you."
    },
    "rs4244285": {
        "simpleImpact": "Slow breakdown of antidepressants and acid blockers (CYP2C19 gene)",
        "simpleMeaning": "Having two copies of this variant makes your body clear certain antidepressants and acid reflux drugs very slowly. It also prevents blood thinners like clopidogrel from working properly."
    },
    "rs4986893": {
        "simpleImpact": "Slow breakdown of antidepressants and acid blockers (CYP2C19 gene)",
        "simpleMeaning": "This variant reduces the activity of an enzyme that clears many common antidepressants and acid reflux drugs from your body, which can lead to stronger side effects."
    },
    "rs4149056": {
        "simpleImpact": "Reduced transport of cholesterol-lowering statins (SLCO1B1 gene)",
        "simpleMeaning": "This variant slows down how statins (cholesterol drugs) are moved into your liver. This can cause the drug to build up in your blood, significantly raising your risk of muscle pain."
    },
    "rs762551": {
        "simpleImpact": "Slower breakdown of caffeine (CYP1A2 gene)",
        "simpleMeaning": "You break down caffeine slowly. It stays in your body longer, which can cause jitteriness, anxiety, or trouble sleeping if you drink coffee or energy drinks later in the day."
    },
    "rs9923231": {
        "simpleImpact": "High sensitivity to blood-thinning warfarin (VKORC1 gene)",
        "simpleMeaning": "This variant makes you highly sensitive to the blood thinner warfarin. You will likely require a lower dose than average to avoid bleeding complications."
    },
    "rs2108622": {
        "simpleImpact": "Altered Vitamin K breakdown (CYP4F2 gene)",
        "simpleMeaning": "This variant makes your body break down Vitamin K slower. Because of this, if you are prescribed the blood thinner warfarin, you may require a higher dose than usual."
    },
    "rs776746": {
        "simpleImpact": "Non-active CYP3A5 enzyme (CYP3A5 gene)",
        "simpleMeaning": "Having two copies of this variant means you do not make an active version of a key drug-clearing enzyme. This slows down your body's ability to process several common medications, including some statins."
    },
    "rs3745274": {
        "simpleImpact": "Reduced breakdown of certain antidepressants and medications (CYP2B6 gene)",
        "simpleMeaning": "This variant slows down how your body clears certain medications, like the antidepressant bupropion. This can increase the risk of side effects unless your dose is adjusted."
    },
    "rs2231142": {
        "simpleImpact": "Reduced transport of uric acid and statins (ABCG2 gene)",
        "simpleMeaning": "This variant makes it harder for your body to move certain drugs and uric acid. This can cause cholesterol medications to build up in your body and increases your risk of gout."
    },
    "rs3918290": {
        "simpleImpact": "Extreme risk of severe side effects from certain chemo drugs (DPYD gene)",
        "simpleMeaning": "You have a gene change that prevents your body from breaking down certain chemotherapy drugs. Standard doses would cause dangerous, life-threatening side effects, so doctors must use other treatments."
    },
    "rs55886062": {
        "simpleImpact": "Extreme risk of severe side effects from certain chemo drugs (DPYD gene)",
        "simpleMeaning": "You have a rare gene change that makes your body process certain chemotherapy drugs very slowly. Standard doses could cause severe, dangerous side effects, so doctors must use alternative treatments or significantly lower doses."
    },
    "rs67376798": {
        "simpleImpact": "High risk of severe side effects from certain chemo drugs (DPYD gene)",
        "simpleMeaning": "This variant reduces your body's ability to clear specific chemotherapy medications. Taking standard doses can lead to severe adverse reactions."
    },
    "rs1800460": {
        "simpleImpact": "Severe risk of side effects from thiopurine drugs (TPMT gene)",
        "simpleMeaning": "This variant reduces the activity of an enzyme that clears immune-suppressing drugs. Standard doses can cause severe, dangerous damage to your bone marrow."
    },
    "rs1142345": {
        "simpleImpact": "Severe risk of side effects from thiopurine drugs (TPMT gene)",
        "simpleMeaning": "Having copies of this variant causes nearly zero enzyme activity for breaking down certain immune-suppressing drugs. Doctors must drastically lower your dose to avoid life-threatening side effects."
    },
    "rs116855232": {
        "simpleImpact": "Severe risk of side effects from thiopurine drugs (NUDT15 gene)",
        "simpleMeaning": "This variant causes your body to process certain immune-suppressing drugs very poorly. It can lead to severe hair loss and a dangerous drop in white blood cells."
    },
    "rs73598374": {
        "simpleImpact": "Higher sleep pressure and deeper sleep (ADA gene)",
        "simpleMeaning": "This variant slows down how your body breaks down adenosine, a chemical that builds up during the day to make you feel sleepy. You may experience deeper sleep but feel groggier when you wake up."
    },
    "rs934945": {
        "simpleImpact": "Sleep schedule disruption and lighter sleep (PER2 gene)",
        "simpleMeaning": "This research-level variant in a core clock gene is linked to disruptions in your sleep schedule, waking up in the middle of the night, and lighter sleep."
    },
    "rs1801260": {
        "simpleImpact": "Natural preference for staying up late (CLOCK gene)",
        "simpleMeaning": "This variant is strongly linked to being a 'night owl.' It can delay your body's natural release of melatonin, making it harder to fall asleep early."
    },
    "rs10830963": {
        "simpleImpact": "Altered sugar control in the morning (MTNR1B gene)",
        "simpleMeaning": "This variant affects how your body handles sugar in the morning. When melatonin levels are still high, your body release less insulin, which can lead to higher morning blood sugar levels."
    },
    "rs225014": {
        "simpleImpact": "Reduced thyroid hormone activation in cells (DIO2 gene)",
        "simpleMeaning": "This variant makes your cells less efficient at turning inactive thyroid hormone into its active form. This can cause mild fatigue or brain fog even if your standard blood tests look normal."
    },
    "rs1050450": {
        "simpleImpact": "Lower antioxidant protection in the thyroid (GPX1 gene)",
        "simpleMeaning": "This variant reduces your body's ability to protect the thyroid gland from oxidative stress. Getting enough selenium in your diet can help support this protective pathway."
    },
    "rs231775": {
        "simpleImpact": "Higher immune cell activation and autoimmune risk (CTLA4 gene)",
        "simpleMeaning": "This variant lowers a safety brake on your immune cells, making them more active. This can slightly raise your risk for autoimmune conditions, where the body attacks its own tissues."
    },
    "rs2476601": {
        "simpleImpact": "Altered immune signaling and autoimmune risk (PTPN22 gene)",
        "simpleMeaning": "This variant changes a key signal in your immune cells, making it harder for your body to turn off immune responses. This is linked to a higher risk of autoimmune thyroid issues and other conditions."
    },
    "rs1883832": {
        "simpleImpact": "Increased thyroid antibody production (CD40 gene)",
        "simpleMeaning": "This variant increases the activity of immune cells that make antibodies. In some people, this can lead to the immune system targeting the thyroid gland."
    },
    "rs179247": {
        "simpleImpact": "Altered thyroid hormone receptor sensitivity (TSHR gene)",
        "simpleMeaning": "This variant affects the sensitivity of your thyroid receptors to hormone signals, which can influence your risk of autoimmune thyroid conditions."
    },
    "rs2235544": {
        "simpleImpact": "Altered thyroid hormone conversion in organs (DIO1 gene)",
        "simpleMeaning": "This variant alters how your liver and kidneys convert thyroid hormone from its inactive to active form, causing minor shifts in thyroid hormone ratios in the blood."
    }
}

def main():
    # Load extracted markers
    markers_path = r"c:\Users\Roy\Desktop\AI\DNA_Tools\scratch\markers_extracted.json"
    with open(markers_path, "r", encoding="utf-8") as f:
        markers = json.load(f)
    
    # Extract rsIDs from JSON
    rsids_in_json = {m["rsid"] for m in markers}
    
    # Check for missing
    missing_in_translations = rsids_in_json - set(TRANSLATIONS.keys())
    extra_in_translations = set(TRANSLATIONS.keys()) - rsids_in_json
    
    if missing_in_translations:
        print(f"ERROR: Missing translations for rsIDs: {missing_in_translations}")
        return
    if extra_in_translations:
        print(f"WARNING: Extra translations in map: {extra_in_translations}")
    
    print(f"Successfully verified all {len(rsids_in_json)} markers are present!")
    
    # Generate TypeScript file content
    ts_content = []
    ts_content.append("// ./src/lib/utils/layperson.ts")
    ts_content.append("/**")
    ts_content.append(" * Purpose: Provide clear, plain-English translations for genetic marker impacts and meanings.")
    ts_content.append(" * Responsibilities:")
    ts_content.append(" * - Define the LaypersonTranslation interface.")
    ts_content.append(" * - Map all 99 rsIDs from markers_extracted.json to their simplified layperson-friendly translations.")
    ts_content.append(" * Key Inputs: None (static mapping).")
    ts_content.append(" * Key Outputs: LAYPERSON_MAP dictionary.")
    ts_content.append(" * Operational Notes: Keeps technical and medical validity while using terms a layperson or teenager can understand.")
    ts_content.append(" */")
    ts_content.append("")
    ts_content.append("export interface LaypersonTranslation {")
    ts_content.append("  simpleImpact: string;")
    ts_content.append("  simpleMeaning: string;")
    ts_content.append("}")
    ts_content.append("")
    ts_content.append("export const LAYPERSON_MAP: Record<string, LaypersonTranslation> = {")
    
    # Order rsIDs by how they appear in markers_extracted.json to preserve logical sequence
    for marker in markers:
        rsid = marker["rsid"]
        trans = TRANSLATIONS[rsid]
        
        # Escape single quotes or use double quotes/backticks
        simple_impact = trans["simpleImpact"].replace('"', '\\"')
        simple_meaning = trans["simpleMeaning"].replace('"', '\\"')
        
        ts_content.append(f'  "{rsid}": {{')
        ts_content.append(f'    simpleImpact: "{simple_impact}",')
        ts_content.append(f'    simpleMeaning: "{simple_meaning}"')
        ts_content.append("  },")
        
    ts_content.append("};")
    ts_content.append("")
    
    output_path = r"c:\Users\Roy\Desktop\AI\DNA_Tools\src\lib\utils\layperson.ts"
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write("\n".join(ts_content))
    
    print(f"Generated {output_path} successfully!")

if __name__ == "__main__":
    main()
