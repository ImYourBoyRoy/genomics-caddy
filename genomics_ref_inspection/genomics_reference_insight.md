# Genomics Reference File Insight

Generated UTC: 2026-07-07T20:25:24.5448228Z
PowerShell: 7.6.3
OS: Microsoft Windows 10.0.26200

## ClinVar

Status: `inspected`

### File

- Path: `C:\Users\Roy\Desktop\AI\DNA_Tools\App\Data\raw_downloads\clinvar\variant_summary.txt`
- Size: 3.67 GB / 3935425275 bytes
- Modified UTC: 2026-06-28T04:28:23.0000000Z

### Detected structure

- Type: `delimited_text`
- Likely delimiter: `tab`
- Column count: 43

#### Columns

- `#AlleleID`
- `Type`
- `Name`
- `GeneID`
- `GeneSymbol`
- `HGNC_ID`
- `ClinicalSignificance`
- `ClinSigSimple`
- `LastEvaluated`
- `RS# (dbSNP)`
- `nsv/esv (dbVar)`
- `RCVaccession`
- `PhenotypeIDS`
- `PhenotypeList`
- `Origin`
- `OriginSimple`
- `Assembly`
- `ChromosomeAccession`
- `Chromosome`
- `Start`
- `Stop`
- `ReferenceAllele`
- `AlternateAllele`
- `Cytogenetic`
- `ReviewStatus`
- `NumberSubmitters`
- `Guidelines`
- `TestedInGTR`
- `OtherIDs`
- `SubmitterCategories`
- `VariationID`
- `PositionVCF`
- `ReferenceAlleleVCF`
- `AlternateAlleleVCF`
- `SomaticClinicalImpact`
- `SomaticClinicalImpactLastEvaluated`
- `ReviewStatusClinicalImpact`
- `Oncogenicity`
- `OncogenicityLastEvaluated`
- `ReviewStatusOncogenicity`
- `SCVsForAggregateGermlineClassification`
- `SCVsForAggregateSomaticClinicalImpact`
- `SCVsForAggregateOncogenicityClassification`

#### Interesting columns

- `#AlleleID`
- `GeneID`
- `GeneSymbol`
- `ClinicalSignificance`
- `LastEvaluated`
- `RS# (dbSNP)`
- `PhenotypeIDS`
- `PhenotypeList`
- `Assembly`
- `ChromosomeAccession`
- `Chromosome`
- `Start`
- `Stop`
- `ReferenceAllele`
- `AlternateAllele`
- `Cytogenetic`
- `ReviewStatus`
- `NumberSubmitters`
- `SubmitterCategories`
- `VariationID`
- `ReferenceAlleleVCF`
- `AlternateAlleleVCF`
- `SomaticClinicalImpact`
- `SomaticClinicalImpactLastEvaluated`
- `ReviewStatusClinicalImpact`
- `OncogenicityLastEvaluated`
- `ReviewStatusOncogenicity`
- `SCVsForAggregateSomaticClinicalImpact`

#### First lines

~~~text
#AlleleID	Type	Name	GeneID	GeneSymbol	HGNC_ID	ClinicalSignificance	ClinSigSimple	LastEvaluated	RS# (dbSNP)	nsv/esv (dbVar)	RCVaccession	PhenotypeIDS	PhenotypeList	Origin	OriginSimple	Assembly	ChromosomeAccession	Chromosome	Start	Stop	ReferenceAllele	AlternateAllele	Cytogenetic	ReviewStatus	NumberSubmitters	Guidelines	TestedInGTR	OtherIDs	SubmitterCategories	VariationID	PositionVCF	ReferenceAlleleVCF	AlternateAlleleVCF	SomaticClinicalImpact	SomaticClinicalImpactLastEvaluated	ReviewStatusClinicalImpact	Oncogenicity	OncogenicityLastEvaluated	ReviewStatusOncogenicity	SCVsForAggregateGermlineClassification	SCVsForAggregateSomaticClinicalImpact	SCVsForAggregateOncogenicityClassification
15041	Indel	NM_014855.3(AP5Z1):c.80_83delinsTGCTGTAAACTGTAACTGTAAA (p.Arg27_Ile28delinsLeuLeuTer)	9907	AP5Z1	HGNC:22197	Pathogenic/Likely pathogenic	1	Dec 17, 2024	397704705	-	RCV000000012|RCV005255549|RCV004998069	MONDO:MONDO:0013342,MedGen:C3150901,OMIM:613647,Orphanet:306511||MedGen:C3661900	Hereditary spastic paraplegia 48|Macular dystrophy with or without extraocular features|not provided	germline;unknown	germline	GRCh37	NC_000007.13	7	4820844	4820847	na	na	7p22.1	criteria provided, multiple submitters, no conflicts	4	-	N	ClinGen:CA215070,OMIM:613653.0001	3	2	4820844	GGAT	TGCTGTAAACTGTAACTGTAAA	-	-	-	-	-	-	SCV001451119|SCV005622007|SCV005909190	-	-
15041	Indel	NM_014855.3(AP5Z1):c.80_83delinsTGCTGTAAACTGTAACTGTAAA (p.Arg27_Ile28delinsLeuLeuTer)	9907	AP5Z1	HGNC:22197	Pathogenic/Likely pathogenic	1	Dec 17, 2024	397704705	-	RCV000000012|RCV005255549|RCV004998069	MONDO:MONDO:0013342,MedGen:C3150901,OMIM:613647,Orphanet:306511||MedGen:C3661900	Hereditary spastic paraplegia 48|Macular dystrophy with or without extraocular features|not provided	germline;unknown	germline	GRCh38	NC_000007.14	7	4781213	4781216	na	na	7p22.1	criteria provided, multiple submitters, no conflicts	4	-	N	ClinGen:CA215070,OMIM:613653.0001	3	2	4781213	GGAT	TGCTGTAAACTGTAACTGTAAA	-	-	-	-	-	-	SCV001451119|SCV005622007|SCV005909190	-	-
15042	Deletion	NM_014855.3(AP5Z1):c.1413_1426del (p.Leu473fs)	9907	AP5Z1	HGNC:22197	Pathogenic	1	Jun 29, 2010	397704709	-	RCV000000013	MONDO:MONDO:0013342,MedGen:C3150901,OMIM:613647,Orphanet:306511	Hereditary spastic paraplegia 48	germline	germline	GRCh37	NC_000007.13	7	4827361	4827374	na	na	7p22.1	no assertion criteria provided	1	-	N	ClinGen:CA215072,OMIM:613653.0002	1	3	4827360	GCTGCTGGACCTGCC	G	-	-	-	-	-	-	SCV000020156	-	-
15042	Deletion	NM_014855.3(AP5Z1):c.1413_1426del (p.Leu473fs)	9907	AP5Z1	HGNC:22197	Pathogenic	1	Jun 29, 2010	397704709	-	RCV000000013	MONDO:MONDO:0013342,MedGen:C3150901,OMIM:613647,Orphanet:306511	Hereditary spastic paraplegia 48	germline	germline	GRCh38	NC_000007.14	7	4787730	4787743	na	na	7p22.1	no assertion criteria provided	1	-	N	ClinGen:CA215072,OMIM:613653.0002	1	3	4787729	GCTGCTGGACCTGCC	G	-	-	-	-	-	-	SCV000020156	-	-
15043	single nucleotide variant	NM_014630.3(ZNF592):c.3136G>A (p.Gly1046Arg)	9640	ZNF592	HGNC:28986	Uncertain significance	0	Jun 29, 2015	150829393	-	RCV000000014	MONDO:MONDO:0033005,MedGen:C4551772,OMIM:251300,Orphanet:2065,Orphanet:83472	Galloway-Mowat syndrome 1	germline	germline	GRCh37	NC_000015.9	15	85342440	85342440	na	na	15q25.3	no assertion criteria provided	1	-	N	ClinGen:CA210674,UniProtKB:Q92610#VAR_064583,OMIM:613624.0001	1	4	85342440	G	A	-	-	-	-	-	-	SCV000020157	-	-
15043	single nucleotide variant	NM_014630.3(ZNF592):c.3136G>A (p.Gly1046Arg)	9640	ZNF592	HGNC:28986	Uncertain significance	0	Jun 29, 2015	150829393	-	RCV000000014	MONDO:MONDO:0033005,MedGen:C4551772,OMIM:251300,Orphanet:2065,Orphanet:83472	Galloway-Mowat syndrome 1	germline	germline	GRCh38	NC_000015.10	15	84799209	84799209	na	na	15q25.3	no assertion criteria provided	1	-	N	ClinGen:CA210674,UniProtKB:Q92610#VAR_064583,OMIM:613624.0001	1	4	84799209	G	A	-	-	-	-	-	-	SCV000020157	-	-
15044	single nucleotide variant	NM_017547.4(FOXRED1):c.694C>T (p.Gln232Ter)	55572	FOXRED1	HGNC:26927	Pathogenic	1	Aug 17, 2025	267606829	-	RCV000000015|RCV000578659|RCV001194045|RCV003390625	MONDO:MONDO:0032624,MedGen:C4748791,OMIM:618241|MedGen:C3661900|MONDO:MONDO:0009723,MedGen:C2931891,OMIM:256000,Orphanet:506|	Mitochondrial complex I deficiency, nuclear type 19|not provided|Leigh syndrome|FOXRED1-related disorder	germline	germline	GRCh37	NC_000011.9	11	126145284	126145284	na	na	11q24.2	criteria provided, multiple submitters, no conflicts	6	-	N	ClinGen:CA113792,OMIM:613622.0001	3	5	126145284	C	T	-	-	-	-	-	-	SCV000680696|SCV001363290|SCV002793147|SCV002982300|SCV004119439	-	-
15044	single nucleotide variant	NM_017547.4(FOXRED1):c.694C>T (p.Gln232Ter)	55572	FOXRED1	HGNC:26927	Pathogenic	1	Aug 17, 2025	267606829	-	RCV000000015|RCV000578659|RCV001194045|RCV003390625	MONDO:MONDO:0032624,MedGen:C4748791,OMIM:618241|MedGen:C3661900|MONDO:MONDO:0009723,MedGen:C2931891,OMIM:256000,Orphanet:506|	Mitochondrial complex I deficiency, nuclear type 19|not provided|Leigh syndrome|FOXRED1-related disorder	germline	germline	GRCh38	NC_000011.10	11	126275389	126275389	na	na	11q24.2	criteria provided, multiple submitters, no conflicts	6	-	N	ClinGen:CA113792,OMIM:613622.0001	3	5	126275389	C	T	-	-	-	-	-	-	SCV000680696|SCV001363290|SCV002793147|SCV002982300|SCV004119439	-	-
15045	single nucleotide variant	NM_017547.4(FOXRED1):c.1289A>G (p.Asn430Ser)	55572	FOXRED1	HGNC:26927	Likely pathogenic	1	Jun 06, 2024	267606830	-	RCV000000016	MONDO:MONDO:0032624,MedGen:C4748791,OMIM:618241	Mitochondrial complex I deficiency, nuclear type 19	germline	germline	GRCh37	NC_000011.9	11	126147412	126147412	na	na	11q24.2	criteria provided, single submitter	2	-	N	ClinGen:CA113794,UniProtKB:Q96CU9#VAR_064571,OMIM:613622.0002	3	6	126147412	A	G	-	-	-	-	-	-	SCV005680614	-	-
15045	single nucleotide variant	NM_017547.4(FOXRED1):c.1289A>G (p.Asn430Ser)	55572	FOXRED1	HGNC:26927	Likely pathogenic	1	Jun 06, 2024	267606830	-	RCV000000016	MONDO:MONDO:0032624,MedGen:C4748791,OMIM:618241	Mitochondrial complex I deficiency, nuclear type 19	germline	germline	GRCh38	NC_000011.10	11	126277517	126277517	na	na	11q24.2	criteria provided, single submitter	2	-	N	ClinGen:CA113794,UniProtKB:Q96CU9#VAR_064571,OMIM:613622.0002	3	6	126277517	A	G	-	-	-	-	-	-	SCV005680614	-	-
15046	single nucleotide variant	NM_025152.3(NUBPL):c.166G>A (p.Gly56Arg)	80224	NUBPL	HGNC:20278	Conflicting classifications of pathogenicity	1	Apr 08, 2025	200401432	-	RCV000196589|RCV000622708|RCV001526454|RCV005055710	MedGen:C3661900|MeSH:D030342,MedGen:C0950123|MONDO:MONDO:0032625,MedGen:C4748792,OMIM:618242|MedGen:CN169374	not provided|Inborn genetic diseases|Mitochondrial complex I deficiency, nuclear type 21|not specified	germline;maternal;paternal	germline	GRCh37	NC_000014.8	14	32031331	32031331	na	na	14q12	criteria provided, conflicting classifications	6	-	N	ClinGen:CA321015,UniProtKB:Q8TB37#VAR_064570,OMIM:613621.0001,ClinVar:7	2	214885	32031331	G	A	-	-	-	-	-	-	SCV000251971|SCV000742093|SCV001736868|SCV003255705|SCV005726475|SCV006101074|SCV006318569	-	-
15046	single nucleotide variant	NM_025152.3(NUBPL):c.166G>A (p.Gly56Arg)	80224	NUBPL	HGNC:20278	Conflicting classifications of pathogenicity	1	Apr 08, 2025	200401432	-	RCV000196589|RCV000622708|RCV001526454|RCV005055710	MedGen:C3661900|MeSH:D030342,MedGen:C0950123|MONDO:MONDO:0032625,MedGen:C4748792,OMIM:618242|MedGen:CN169374	not provided|Inborn genetic diseases|Mitochondrial complex I deficiency, nuclear type 21|not specified	germline;maternal;paternal	germline	GRCh38	NC_000014.9	14	31562125	31562125	na	na	14q12	criteria provided, conflicting classifications	6	-	N	ClinGen:CA321015,UniProtKB:Q8TB37#VAR_064570,OMIM:613621.0001,ClinVar:7	2	214885	31562125	G	A	-	-	-	-	-	-	SCV000251971|SCV000742093|SCV001736868|SCV003255705|SCV005726475|SCV006101074|SCV006318569	-	-
15048	single nucleotide variant	NM_000410.4(HFE):c.845G>A (p.Cys282Tyr)	3077	HFE	HGNC:4886	Pathogenic/Pathogenic, low penetrance; risk factor	1	Feb 13, 2026	1800562	-	RCV000000019|RCV000178096|RCV000210820|RCV000308358|RCV000414811|RCV001248830|RCV001270034|RCV002280089|RCV001731264|RCV002512585|RCV003224084|RCV003493406|RCV005621842	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:C3661900|MONDO:MONDO:0015356,MeSH:D009386,MedGen:C0027672,Orphanet:140162|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|Human Phenotype Ontology:HP:0010473,MedGen:C0151861;Human Phenotype Ontology:HP:0000992,Human Phenotype Ontology:HP:0005594,Human Phenotype Ontology:HP:0006831,Human Phenotype Ontology:HP:0007538,MONDO:MONDO:0005434,MedGen:C0349506|MedGen:C6022505,Orphanet:220489|7 conditions||Human Phenotype Ontology:HP:0001638,MONDO:MONDO:0004994,MedGen:C0878544,Orphanet:167848|MeSH:D030342,MedGen:C0950123|6 conditions|MONDO:MONDO:0019257,MedGen:C0268060,Orphanet:79230|Human Phenotype Ontology:HP:0100634,MONDO:MONDO:0019496,MedGen:C0206754,Orphanet:877	Hemochromatosis type 1|not provided|Hereditary cancer-predisposing syndrome|Hereditary hemochromatosis|Porphyrinuria;Cutaneous photosensitivity|Bronze diabetes|7 conditions|HFE-related disorder|Cardiomyopathy|Inborn genetic diseases|6 conditions|Juvenile hemochromatosis|Neuroendocrine neoplasm	biparental;germline;unknown	germline	GRCh37	NC_000006.11	6	26093141	26093141	na	na	6p22.2	criteria provided, multiple submitters, no conflicts	58	ACMG2021,ACMG2022	Y	UniProtKB:Q30201#VAR_004398,OMIM:613609.0001,ClinGen:CA113795	3	9	26093141	G	A	-	-	-	-	-	-	SCV000151394|SCV000206975|SCV000219175|SCV000221190|SCV000223934|SCV000267038|SCV000329362|SCV000461887|SCV000839959|SCV000883106|SCV001137062|SCV001194044|SCV001246053|SCV001251531|SCV001448752|SCV001519562|SCV001523198|SCV001905583|SCV001984982|SCV002028313|SCV002044430|SCV002061285|SCV002502491|SCV002525758|SCV002550674|SCV002568182|SCV002576300|SCV002580992|SCV003702847|SCV003799234|SCV003920032|SCV003925227|SCV004177020|SCV004803175|SCV004812520|SCV005086593|SCV005198438|SCV005382109|SCV005382430|SCV005415942|SCV005440717|SCV005669471|SCV006583014|SCV007095984|SCV007520912|SCV007540682	-	-
15048	single nucleotide variant	NM_000410.4(HFE):c.845G>A (p.Cys282Tyr)	3077	HFE	HGNC:4886	Pathogenic/Pathogenic, low penetrance; risk factor	1	Feb 13, 2026	1800562	-	RCV000000019|RCV000178096|RCV000210820|RCV000308358|RCV000414811|RCV001248830|RCV001270034|RCV002280089|RCV001731264|RCV002512585|RCV003224084|RCV003493406|RCV005621842	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:C3661900|MONDO:MONDO:0015356,MeSH:D009386,MedGen:C0027672,Orphanet:140162|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|Human Phenotype Ontology:HP:0010473,MedGen:C0151861;Human Phenotype Ontology:HP:0000992,Human Phenotype Ontology:HP:0005594,Human Phenotype Ontology:HP:0006831,Human Phenotype Ontology:HP:0007538,MONDO:MONDO:0005434,MedGen:C0349506|MedGen:C6022505,Orphanet:220489|7 conditions||Human Phenotype Ontology:HP:0001638,MONDO:MONDO:0004994,MedGen:C0878544,Orphanet:167848|MeSH:D030342,MedGen:C0950123|6 conditions|MONDO:MONDO:0019257,MedGen:C0268060,Orphanet:79230|Human Phenotype Ontology:HP:0100634,MONDO:MONDO:0019496,MedGen:C0206754,Orphanet:877	Hemochromatosis type 1|not provided|Hereditary cancer-predisposing syndrome|Hereditary hemochromatosis|Porphyrinuria;Cutaneous photosensitivity|Bronze diabetes|7 conditions|HFE-related disorder|Cardiomyopathy|Inborn genetic diseases|6 conditions|Juvenile hemochromatosis|Neuroendocrine neoplasm	biparental;germline;unknown	germline	GRCh38	NC_000006.12	6	26092913	26092913	na	na	6p22.2	criteria provided, multiple submitters, no conflicts	58	ACMG2021,ACMG2022	Y	UniProtKB:Q30201#VAR_004398,OMIM:613609.0001,ClinGen:CA113795	3	9	26092913	G	A	-	-	-	-	-	-	SCV000151394|SCV000206975|SCV000219175|SCV000221190|SCV000223934|SCV000267038|SCV000329362|SCV000461887|SCV000839959|SCV000883106|SCV001137062|SCV001194044|SCV001246053|SCV001251531|SCV001448752|SCV001519562|SCV001523198|SCV001905583|SCV001984982|SCV002028313|SCV002044430|SCV002061285|SCV002502491|SCV002525758|SCV002550674|SCV002568182|SCV002576300|SCV002580992|SCV003702847|SCV003799234|SCV003920032|SCV003925227|SCV004177020|SCV004803175|SCV004812520|SCV005086593|SCV005198438|SCV005382109|SCV005382430|SCV005415942|SCV005440717|SCV005669471|SCV006583014|SCV007095984|SCV007520912|SCV007540682	-	-
15049	single nucleotide variant	NM_000410.4(HFE):c.187C>G (p.His63Asp)	3077	HFE	HGNC:4886	Conflicting classifications of pathogenicity; other	1	Apr 27, 2026	1799945	-	RCV000000026|RCV000175607|RCV000394716|RCV000763144|RCV000991133|RCV000844708|RCV001248831|RCV002272003|RCV001731265|RCV004584302|RCV005621843	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:C3661900|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|6 conditions|MONDO:MONDO:0009061,MedGen:C0010674,OMIM:219700,Orphanet:586|MedGen:CN169374|MedGen:C6022505,Orphanet:220489|MONDO:MONDO:0008297,MedGen:C0162532,OMIM:176200,Orphanet:79473|Human Phenotype Ontology:HP:0001638,MONDO:MONDO:0004994,MedGen:C0878544,Orphanet:167848||Human Phenotype Ontology:HP:0100634,MONDO:MONDO:0019496,MedGen:C0206754,Orphanet:877	Hemochromatosis type 1|not provided|Hereditary hemochromatosis|6 conditions|Cystic fibrosis|not specified|Bronze diabetes|Variegate porphyria|Cardiomyopathy|See cases|Neuroendocrine neoplasm	biparental;germline;inherited;unknown	germline	GRCh37	NC_000006.11	6	26091179	26091179	na	na	6p22.2	criteria provided, conflicting classifications	60	ACMG2021,ACMG2022	Y	ClinGen:CA113797,UniProtKB:Q30201#VAR_004396,OMIM:613609.0002	3	10	26091179	C	G	-	-	-	-	-	-	SCV000219176|SCV000223933|SCV000227124|SCV000461883|SCV000577565|SCV000693430|SCV000893709|SCV001137061|SCV001154674|SCV001194094|SCV001251532|SCV001368348|SCV001519563|SCV001523197|SCV001715880|SCV001905582|SCV001984998|SCV002028310|SCV002038504|SCV002499222|SCV002502480|SCV002506442|SCV002556586|SCV002568070|SCV002576301|SCV002583554|SCV002769510|SCV004045959|SCV004046529|SCV004183355|SCV004801387|SCV004803201|SCV004806939|SCV004847117|SCV005061024|SCV005198437|SCV005417670|SCV005669467|SCV006107443|SCV006303008|SCV006324874|SCV006582525|SCV007520911|SCV007540681|SCV007580477|SCV007596800|SCV007598646	-	-
15049	single nucleotide variant	NM_000410.4(HFE):c.187C>G (p.His63Asp)	3077	HFE	HGNC:4886	Conflicting classifications of pathogenicity; other	1	Apr 27, 2026	1799945	-	RCV000000026|RCV000175607|RCV000394716|RCV000763144|RCV000991133|RCV000844708|RCV001248831|RCV002272003|RCV001731265|RCV004584302|RCV005621843	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:C3661900|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|6 conditions|MONDO:MONDO:0009061,MedGen:C0010674,OMIM:219700,Orphanet:586|MedGen:CN169374|MedGen:C6022505,Orphanet:220489|MONDO:MONDO:0008297,MedGen:C0162532,OMIM:176200,Orphanet:79473|Human Phenotype Ontology:HP:0001638,MONDO:MONDO:0004994,MedGen:C0878544,Orphanet:167848||Human Phenotype Ontology:HP:0100634,MONDO:MONDO:0019496,MedGen:C0206754,Orphanet:877	Hemochromatosis type 1|not provided|Hereditary hemochromatosis|6 conditions|Cystic fibrosis|not specified|Bronze diabetes|Variegate porphyria|Cardiomyopathy|See cases|Neuroendocrine neoplasm	biparental;germline;inherited;unknown	germline	GRCh38	NC_000006.12	6	26090951	26090951	na	na	6p22.2	criteria provided, conflicting classifications	60	ACMG2021,ACMG2022	Y	ClinGen:CA113797,UniProtKB:Q30201#VAR_004396,OMIM:613609.0002	3	10	26090951	C	G	-	-	-	-	-	-	SCV000219176|SCV000223933|SCV000227124|SCV000461883|SCV000577565|SCV000693430|SCV000893709|SCV001137061|SCV001154674|SCV001194094|SCV001251532|SCV001368348|SCV001519563|SCV001523197|SCV001715880|SCV001905582|SCV001984998|SCV002028310|SCV002038504|SCV002499222|SCV002502480|SCV002506442|SCV002556586|SCV002568070|SCV002576301|SCV002583554|SCV002769510|SCV004045959|SCV004046529|SCV004183355|SCV004801387|SCV004803201|SCV004806939|SCV004847117|SCV005061024|SCV005198437|SCV005417670|SCV005669467|SCV006107443|SCV006303008|SCV006324874|SCV006582525|SCV007520911|SCV007540681|SCV007580477|SCV007596800|SCV007598646	-	-
15050	single nucleotide variant	NM_000410.4(HFE):c.193A>T (p.Ser65Cys)	3077	HFE	HGNC:4886	Conflicting classifications of pathogenicity	1	Feb 03, 2026	1800730	-	RCV000000028|RCV000290779|RCV000764641|RCV000998547|RCV001328435|RCV004532264|RCV003224085	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|6 conditions|MedGen:C3661900|MedGen:CN169374||6 conditions	Hemochromatosis type 1|Hereditary hemochromatosis|6 conditions|not provided|not specified|HFE-related disorder|6 conditions	germline;unknown	germline	GRCh37	NC_000006.11	6	26091185	26091185	na	na	6p22.2	criteria provided, conflicting classifications	16	ACMG2021,ACMG2022	Y	UniProtKB:Q30201#VAR_004397,OMIM:613609.0003,ClinGen:CA339778	3	11	26091185	A	T	-	-	-	-	-	-	SCV000254532|SCV000895749|SCV001154675|SCV001519564|SCV001840774|SCV002044432|SCV002517178|SCV003920031|SCV004242530|SCV004848231|SCV005086727	-	-
15050	single nucleotide variant	NM_000410.4(HFE):c.193A>T (p.Ser65Cys)	3077	HFE	HGNC:4886	Conflicting classifications of pathogenicity	1	Feb 03, 2026	1800730	-	RCV000000028|RCV000290779|RCV000764641|RCV000998547|RCV001328435|RCV004532264|RCV003224085	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200|6 conditions|MedGen:C3661900|MedGen:CN169374||6 conditions	Hemochromatosis type 1|Hereditary hemochromatosis|6 conditions|not provided|not specified|HFE-related disorder|6 conditions	germline;unknown	germline	GRCh38	NC_000006.12	6	26090957	26090957	na	na	6p22.2	criteria provided, conflicting classifications	16	ACMG2021,ACMG2022	Y	UniProtKB:Q30201#VAR_004397,OMIM:613609.0003,ClinGen:CA339778	3	11	26090957	A	T	-	-	-	-	-	-	SCV000254532|SCV000895749|SCV001154675|SCV001519564|SCV001840774|SCV002044432|SCV002517178|SCV003920031|SCV004242530|SCV004848231|SCV005086727	-	-
15051	single nucleotide variant	NM_000410.4(HFE):c.314T>C (p.Ile105Thr)	3077	HFE	HGNC:4886	Uncertain significance	1	Aug 25, 2021	28934596	-	RCV000000029|RCV001322296	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200	Hemochromatosis type 1|Hereditary hemochromatosis	germline	germline	GRCh37	NC_000006.11	6	26091306	26091306	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA280941,UniProtKB:Q30201#VAR_008730,OMIM:613609.0009	3	12	26091306	T	C	-	-	-	-	-	-	SCV001513159	-	-
15051	single nucleotide variant	NM_000410.4(HFE):c.314T>C (p.Ile105Thr)	3077	HFE	HGNC:4886	Uncertain significance	1	Aug 25, 2021	28934596	-	RCV000000029|RCV001322296	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MONDO:MONDO:0006507,MedGen:C0392514,OMIM:PS235200	Hemochromatosis type 1|Hereditary hemochromatosis	germline	germline	GRCh38	NC_000006.12	6	26091078	26091078	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA280941,UniProtKB:Q30201#VAR_008730,OMIM:613609.0009	3	12	26091078	T	C	-	-	-	-	-	-	SCV001513159	-	-
15052	single nucleotide variant	NM_000410.4(HFE):c.277G>C (p.Gly93Arg)	3077	HFE	HGNC:4886	Uncertain significance	1	Jul 24, 2024	28934597	-	RCV000000030|RCV004700171	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:CN169374	Hemochromatosis type 1|not specified	germline	germline	GRCh37	NC_000006.11	6	26091269	26091269	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA280943,UniProtKB:Q30201#VAR_008729,OMIM:613609.0010	3	13	26091269	G	C	-	-	-	-	-	-	SCV005202844	-	-
15052	single nucleotide variant	NM_000410.4(HFE):c.277G>C (p.Gly93Arg)	3077	HFE	HGNC:4886	Uncertain significance	1	Jul 24, 2024	28934597	-	RCV000000030|RCV004700171	MONDO:MONDO:0021001,MedGen:C3469186,OMIM:235200,Orphanet:465508|MedGen:CN169374	Hemochromatosis type 1|not specified	germline	germline	GRCh38	NC_000006.12	6	26091041	26091041	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA280943,UniProtKB:Q30201#VAR_008729,OMIM:613609.0010	3	13	26091041	G	C	-	-	-	-	-	-	SCV005202844	-	-
15053	single nucleotide variant	NM_000410.4(HFE):c.892+48G>A	3077	HFE	HGNC:4886	Benign	0	Sep 11, 2018	1800758	-	RCV000000031|RCV001618204	|MedGen:C3661900	HFE INTRONIC POLYMORPHISM|not provided	germline	germline	GRCh37	NC_000006.11	6	26093236	26093236	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA113800,OMIM:613609.0004	3	14	26093236	G	A	-	-	-	-	-	-	SCV001844562	-	-
15053	single nucleotide variant	NM_000410.4(HFE):c.892+48G>A	3077	HFE	HGNC:4886	Benign	0	Sep 11, 2018	1800758	-	RCV000000031|RCV001618204	|MedGen:C3661900	HFE INTRONIC POLYMORPHISM|not provided	germline	germline	GRCh38	NC_000006.12	6	26093008	26093008	na	na	6p22.2	criteria provided, single submitter	2	ACMG2021,ACMG2022	N	ClinGen:CA113800,OMIM:613609.0004	3	14	26093008	G	A	-	-	-	-	-	-	SCV001844562	-	-
~~~

## dbSNP

Status: `inspected`

### File

- Path: `C:\Users\Roy\Desktop\AI\DNA_Tools\App\Data\raw_downloads\dbsnp\refsnp-merged.json`
- Size: 19.81 GB / 21272715079 bytes
- Modified UTC: 2026-07-07T07:42:04.4555643Z

### Detected structure

- Type: `json_or_json_like`
- First non-whitespace char: `{`
- Likely NDJSON: `True`
- First line parse success: `True`
- First object parse success: `True`
- Tail ends with JSON array: `False`
- Tail ends with JSON object: `True`

#### First line keys

- `refsnp_id`
- `create_date`
- `last_update_date`
- `last_update_build_id`
- `dbsnp1_merges`
- `citations`
- `lost_obs_movements`
- `present_obs_movements`
- `merged_snapshot_data`
- `mane_select_ids`

#### First object keys

- `refsnp_id`
- `create_date`
- `last_update_date`
- `last_update_build_id`
- `dbsnp1_merges`
- `citations`
- `lost_obs_movements`
- `present_obs_movements`
- `merged_snapshot_data`
- `mane_select_ids`

#### Common keys in first sample

- `dbsnp1_merges`: 2895
- `merged_into`: 2895
- `citations`: 2895
- `lost_obs_movements`: 2895
- `create_date`: 2895
- `refsnp_id`: 2895
- `present_obs_movements`: 2895
- `last_update_date`: 2895
- `proxy_time`: 2895
- `proxy_build_id`: 2895
- `merged_snapshot_data`: 2895
- `last_update_build_id`: 2895
- `mane_select_ids`: 2894
- `merge_date`: 984
- `merged_rsid`: 984
- `revision`: 984
- `position`: 54
- `inserted_sequence`: 54
- `seq_id`: 54
- `deleted_sequence`: 54
- `type`: 36
- `value`: 36
- `observation`: 18
- `rsids_in_cur_release`: 18
- `allele_in_prev_release`: 18
- `allele_in_cur_release`: 18
- `component_ids`: 18

#### First lines

~~~text
{"refsnp_id":"332","create_date":"2000-09-19T17:02Z","last_update_date":"2011-01-11T17:12Z","last_update_build_id":"133","dbsnp1_merges":[{"merged_rsid":"33969899","revision":"127","merge_date":"2006-10-13T20:01Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2011-05-20T17:31Z","proxy_build_id":"133","merged_into":["121909001"]},"mane_select_ids":[]}
{"refsnp_id":"668","create_date":"2000-09-19T17:02Z","last_update_date":"2014-08-21T18:14Z","last_update_build_id":"136","dbsnp1_merges":[{"merged_rsid":"1131009","revision":"102","merge_date":"2002-01-4T16:38Z"},{"merged_rsid":"2228288","revision":"102","merge_date":"2002-01-4T16:38Z"},{"merged_rsid":"3190515","revision":"108","merge_date":"2002-10-9T00:17Z"},{"merged_rsid":"4072030","revision":"123","merge_date":"2004-10-8T09:15Z"},{"merged_rsid":"16947745","revision":"123","merge_date":"2004-10-8T09:15Z"},{"merged_rsid":"17543802","revision":"123","merge_date":"2004-10-8T09:15Z"},{"merged_rsid":"28933978","revision":"130","merge_date":"2008-05-26T13:19Z"},{"merged_rsid":"41313213","revision":"130","merge_date":"2008-05-26T13:19Z"},{"merged_rsid":"52813088","revision":"128","merge_date":"2007-09-21T16:01Z"},{"merged_rsid":"56552296","revision":"130","merge_date":"2008-05-26T13:19Z"},{"merged_rsid":"150163973","revision":"142","merge_date":"2014-08-21T16:14Z"}],"citations":[19055786,19406964,20031567,21155722,22646485,23906684],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2014-08-26T00:20Z","proxy_build_id":"136","merged_into":["281865545"]},"mane_select_ids":[]}
{"refsnp_id":"840","create_date":"2000-08-22T15:29Z","last_update_date":"2000-08-22T15:29Z","last_update_build_id":"85","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-09-19T14:28Z","proxy_build_id":"85","merged_into":["715"]},"mane_select_ids":[]}
{"refsnp_id":"905","create_date":"2000-08-22T15:29Z","last_update_date":"2000-08-22T15:29Z","last_update_build_id":"85","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-09-19T14:28Z","proxy_build_id":"85","merged_into":["716"]},"mane_select_ids":[]}
{"refsnp_id":"1086","create_date":"2000-09-19T17:02Z","last_update_date":"2004-10-4T13:37Z","last_update_build_id":"123","dbsnp1_merges":[{"merged_rsid":"11044","revision":"87","merge_date":"2000-10-23T17:08Z"},{"merged_rsid":"3189863","revision":"106","merge_date":"2002-07-3T13:35Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2004-10-8T09:45Z","proxy_build_id":"123","merged_into":["940"]},"mane_select_ids":[]}
{"refsnp_id":"1234","create_date":"2000-09-19T17:02Z","last_update_date":"2004-10-4T13:37Z","last_update_build_id":"126","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2006-03-11T07:22Z","proxy_build_id":"126","merged_into":["1067"]},"mane_select_ids":[]}
{"refsnp_id":"1986","create_date":"2000-08-22T15:29Z","last_update_date":"2000-08-22T15:29Z","last_update_build_id":"86","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-6T02:07Z","proxy_build_id":"86","merged_into":["1164"]},"mane_select_ids":[]}
{"refsnp_id":"2421","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"92","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-01-19T18:17Z","proxy_build_id":"92","merged_into":["2407"]},"mane_select_ids":[]}
{"refsnp_id":"3001","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["2032"]},"mane_select_ids":[]}
{"refsnp_id":"3244","create_date":"2000-09-19T17:02Z","last_update_date":"2001-01-30T12:09Z","last_update_build_id":"108","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2002-10-9T00:17Z","proxy_build_id":"108","merged_into":["2753"]},"mane_select_ids":[]}
{"refsnp_id":"3320","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["2410"]},"mane_select_ids":[]}
{"refsnp_id":"3441","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"92","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-01-19T18:17Z","proxy_build_id":"92","merged_into":["2584"]},"mane_select_ids":[]}
{"refsnp_id":"3451","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"92","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-01-19T18:17Z","proxy_build_id":"92","merged_into":["2773"]},"mane_select_ids":[]}
{"refsnp_id":"3676","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["1025"]},"mane_select_ids":[]}
{"refsnp_id":"3706","create_date":"2000-09-19T17:02Z","last_update_date":"2001-01-30T12:09Z","last_update_build_id":"94","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-04-12T18:39Z","proxy_build_id":"94","merged_into":["3606"]},"mane_select_ids":[]}
{"refsnp_id":"3707","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["711"]},"mane_select_ids":[]}
{"refsnp_id":"3773","create_date":"2000-09-19T17:02Z","last_update_date":"2004-04-7T14:40Z","last_update_build_id":"123","dbsnp1_merges":[{"merged_rsid":"1043057","revision":"108","merge_date":"2002-10-9T00:17Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2004-09-24T19:24Z","proxy_build_id":"123","merged_into":["536"]},"mane_select_ids":[]}
{"refsnp_id":"3789","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[{"merged_rsid":"10674","revision":"85","merge_date":"2000-09-19T14:28Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["1228"]},"mane_select_ids":[]}
{"refsnp_id":"3798","create_date":"2000-08-22T15:29Z","last_update_date":"2000-08-22T15:29Z","last_update_build_id":"86","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-6T02:07Z","proxy_build_id":"86","merged_into":["1164"]},"mane_select_ids":[]}
{"refsnp_id":"4182","create_date":"2000-09-19T17:02Z","last_update_date":"2017-02-15T16:06Z","last_update_build_id":"136","dbsnp1_merges":[{"merged_rsid":"3217272","revision":"138","merge_date":"2013-05-16T02:16Z"},{"merged_rsid":"143696344","revision":"135","merge_date":"2011-09-17T04:07Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2017-02-27T11:33Z","proxy_build_id":"136","merged_into":["397794472"]},"mane_select_ids":[]}
{"refsnp_id":"4202","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"92","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-01-19T18:17Z","proxy_build_id":"92","merged_into":["1614"]},"mane_select_ids":[]}
{"refsnp_id":"4255","create_date":"2000-09-19T17:02Z","last_update_date":"2008-03-28T13:52Z","last_update_build_id":"130","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2008-05-26T03:24Z","proxy_build_id":"130","merged_into":["750"]},"mane_select_ids":[]}
{"refsnp_id":"4516","create_date":"2000-10-6T04:23Z","last_update_date":"2000-10-6T04:23Z","last_update_build_id":"87","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2000-10-23T17:08Z","proxy_build_id":"87","merged_into":["4463"]},"mane_select_ids":[]}
{"refsnp_id":"4540","create_date":"2000-09-19T17:02Z","last_update_date":"2004-10-4T13:37Z","last_update_build_id":"123","dbsnp1_merges":[{"merged_rsid":"1137478","revision":"100","merge_date":"2001-09-28T14:46Z"}],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2004-10-8T04:27Z","proxy_build_id":"123","merged_into":["4536"]},"mane_select_ids":[]}
{"refsnp_id":"4542","create_date":"2000-09-19T17:02Z","last_update_date":"2001-01-30T12:10Z","last_update_build_id":"100","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2001-09-28T14:46Z","proxy_build_id":"100","merged_into":["3097"]},"mane_select_ids":[]}
~~~

#### Tail preview

~~~text
ted_sequence":"C"}},{"component_ids":[{"type":"frequency","value":"1000Genomes.2:42687052"},{"type":"subsnp","value":"5555161117"}],"observation":{"seq_id":"NC_000006.12","position":72404186,"deleted_sequence":"C","inserted_sequence":"T"},"allele_in_cur_release":{"seq_id":"NC_000006.12","position":72404186,"deleted_sequence":"C","inserted_sequence":"T"},"rsids_in_cur_release":["2127299749"],"allele_in_prev_release":{"seq_id":"NC_000006.12","position":72404186,"deleted_sequence":"C","inserted_sequence":"T"}}],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2024-11-4T20:32Z","proxy_build_id":"157","merged_into":["2127299749"]},"mane_select_ids":[]}
{"refsnp_id":"2154572637","create_date":"2022-10-17T12:58Z","last_update_date":"2024-11-4T16:29Z","last_update_build_id":"157","dbsnp1_merges":[],"citations":[],"lost_obs_movements":[{"component_ids":[{"type":"frequency","value":"1000Genomes.2:66452452"},{"type":"subsnp","value":"5578926517"}],"observation":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"G"},"allele_in_cur_release":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"G"},"rsids_in_cur_release":["2131710699"],"allele_in_prev_release":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"G"}},{"component_ids":[{"type":"frequency","value":"1000Genomes.2:66452452"},{"type":"subsnp","value":"5578926517"}],"observation":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"A"},"allele_in_cur_release":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"A"},"rsids_in_cur_release":["2131710699"],"allele_in_prev_release":{"seq_id":"NC_000010.11","position":77659005,"deleted_sequence":"G","inserted_sequence":"A"}}],"present_obs_movements":[],"merged_snapshot_data":{"proxy_time":"2024-11-4T16:29Z","proxy_build_id":"157","merged_into":["2131710699"]},"mane_select_ids":[]}

~~~

