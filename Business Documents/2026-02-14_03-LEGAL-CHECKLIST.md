# Sassy Browser -- Legal Checklist

**Sassy Consulting LLC | February 2026**

---

## Entity & Formation

- [ ] Confirm LLC is active and in good standing with Secretary of State
- [ ] Obtain certified copy of Articles of Organization
- [ ] Obtain EIN from IRS (if not already done)
- [ ] Draft/update Operating Agreement (especially for bringing on co-founders or investors)
- [ ] Consider converting to C-Corp (Delaware) if pursuing VC funding (most VCs require it)
- [ ] Register to do business in states where employees/contractors are located
- [ ] Obtain business licenses required by city/county

---

## Intellectual Property

### Patents
- [ ] Engage patent attorney
- [ ] Conduct prior art searches (4 inventions identified -- see PATENT-STRATEGY.md)
- [ ] File provisional patent applications ($320 each, micro entity)
- [ ] Calendar 12-month conversion deadlines
- [ ] Document all invention dates, development logs, and contributor records

### Trademarks
- [ ] Search USPTO TESS for "Sassy Browser" availability
- [ ] File trademark application for "Sassy Browser" (word mark, Class 9)
- [ ] File trademark application for logo (design mark, Class 9)
- [ ] File trademark for tagline "Your browser. Your data. Always."
- [ ] Monitor for infringing marks after filing
- [ ] Register domain variations (sassybrowser.com, .io, .dev, .app)

### Copyright
- [ ] Register source code with US Copyright Office (Form TX, $65)
- [ ] Add copyright headers to all source files
- [ ] Include LICENSE file in repository root
- [ ] Ensure all contributor code is properly licensed

### Open Source Compliance
- [ ] Audit all 96 crate dependencies for license compatibility
- [ ] Create NOTICES file listing all third-party licenses
- [ ] Verify compliance with each license type:
  - MIT: include copyright notice (most crates)
  - Apache-2.0: include NOTICE file, copyright, license text
  - BSD: include copyright notice
  - MPL-2.0: file-level copyleft (check for any)
  - GPL: incompatible with proprietary -- verify none are used
- [ ] Choose license for Sassy Browser itself:
  - Option A: Proprietary (if keeping source closed)
  - Option B: BSL (Business Source License) -- open source after delay
  - Option C: AGPL (strong copyleft, deters competitors)
  - Option D: MIT/Apache-2.0 (permissive, maximum adoption)

---

## Privacy & Data Protection

### Privacy Policy
- [ ] Draft privacy policy for browser (state: we collect nothing)
- [ ] Draft privacy policy for website (standard web analytics notice)
- [ ] Include COPPA compliance language (family profiles serve children)
- [ ] Include GDPR compliance statement (no EU data processing)
- [ ] Include CCPA compliance statement (no California consumer data)
- [ ] Publish privacy policy at sassyconsultingllc.com/privacy
- [ ] Link privacy policy from browser settings

### Terms of Service
- [ ] Draft browser Terms of Service / EULA
- [ ] Include limitation of liability
- [ ] Include warranty disclaimer (AS IS)
- [ ] Include dispute resolution clause (arbitration vs. litigation)
- [ ] Include jurisdiction and governing law
- [ ] Draft acceptable use policy for extension marketplace
- [ ] Draft contributor terms for open source contributions (CLA)

### Data Handling
- [ ] Document data flow diagrams (everything local -- this is our strength)
- [ ] Prepare Data Processing Agreement template (for enterprise customers)
- [ ] Create data retention policy document
- [ ] Document encryption specifications for compliance audits

---

## Employment & Contractor Law

### Before First Hire
- [ ] Register for state unemployment insurance
- [ ] Register for workers' compensation insurance
- [ ] Set up payroll (Gusto, Rippling, or Deel)
- [ ] Draft employment agreement template
- [ ] Draft Invention Assignment Agreement (employees assign IP to company)
- [ ] Draft Confidentiality/NDA template
- [ ] Draft contractor agreement template (for pre-hire contractors)
- [ ] Verify worker classification (W-2 employee vs 1099 contractor)

### Equity & Compensation
- [ ] Establish stock option pool (if C-Corp) or unit plan (if LLC)
- [ ] Draft stock option agreement template
- [ ] Determine vesting schedule (standard: 4 years, 1-year cliff)
- [ ] Get 409A valuation before issuing options (C-Corp requirement)
- [ ] Document all equity grants and exercises

---

## Fundraising Legal

### SAFE / Convertible Note
- [ ] Choose instrument: SAFE (YC standard) vs Convertible Note
- [ ] Draft or customize SAFE agreement (YC template available at ycombinator.com)
- [ ] Set valuation cap and discount rate
- [ ] Determine pro rata rights
- [ ] Create cap table tracking all ownership
- [ ] File Form D with SEC (Regulation D exemption) after closing
- [ ] Comply with state blue sky laws for each investor's state

### Investor Communications
- [ ] Create NDA template for sharing detailed materials
- [ ] Create data room with access controls
- [ ] Set up investor update template (monthly or quarterly)
- [ ] Document all investor representations and warranties

---

## Insurance

- [ ] General liability insurance
- [ ] Professional liability / E&O insurance
- [ ] Cyber liability insurance (especially important for a browser company)
- [ ] D&O insurance (Directors & Officers -- when you have a board)
- [ ] Key person insurance on founder

---

## Regulatory Compliance

### Export Controls
- [ ] Verify encryption software export compliance (EAR/BIS)
- [ ] ChaCha20-Poly1305 and other crypto may require export classification
- [ ] File encryption classification request with BIS if distributing internationally
- [ ] Note: open source encryption has exemptions under EAR 740.13(e)

### Accessibility
- [ ] Review WCAG 2.1 compliance for browser UI
- [ ] Ensure screen reader compatibility
- [ ] Document accessibility features for government/enterprise customers

### COPPA (Children's Online Privacy Protection Act)
- [ ] Family profiles serve users under 13 -- COPPA applies
- [ ] Verify: no data collection from children (our architecture guarantees this)
- [ ] Draft COPPA compliance statement
- [ ] Consider: COPPA Safe Harbor program participation

---

## Ongoing Legal Maintenance

### Monthly
- [ ] Review and update cap table
- [ ] Track all IP filings and deadlines
- [ ] Review contractor/vendor agreements

### Quarterly
- [ ] State tax filings
- [ ] Review insurance coverage
- [ ] Update investor on legal status

### Annually
- [ ] Annual LLC/Corp report filing with state
- [ ] Federal and state tax returns
- [ ] Renew business licenses
- [ ] Review and update all policies
- [ ] IP portfolio review (patents, trademarks, domains)
- [ ] 409A valuation update (if C-Corp with options)

---

## Estimated Legal Costs (Year 1)

| Item | Estimated Cost |
|------|---------------:|
| Startup attorney (formation, fundraising) | $5,000 - $15,000 |
| Patent attorney (4 provisionals) | $8,000 - $15,000 |
| Trademark filings (3 marks) | $750 - $1,500 |
| Copyright registration | $195 |
| Open source license audit | $2,000 - $5,000 |
| Employment agreements drafting | $2,000 - $5,000 |
| Insurance (annual) | $3,000 - $8,000 |
| **Total** | **$21,000 - $50,000** |

Budget $30K for legal in pre-seed (already allocated in investment ask).

---

**Document Version:** 1.0 | February 14, 2026
**Sassy Consulting LLC**

**Disclaimer:** This checklist is for planning purposes. Consult qualified attorneys for all legal matters.
