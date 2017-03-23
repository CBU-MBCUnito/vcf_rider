use super::mutations;
use std::collections::VecDeque;

// Used to classify indels and snps in groups: SNPs will be tagged as "Manage" while
// indels with Ins or Del (if this group has their alternative allele).
// String is the sequence that needs to be inserted for ins, usize is the coord inside the window
// and u32 the length of deletions.
#[derive(Eq, PartialEq, Debug)]
pub enum MutationClass {
    Manage(usize),
    Ins(Vec<u8>, usize),
    Del(u64, usize),
    Reference
}

#[allow(dead_code)]
pub struct IndelRider {
    groups: Vec<Vec<u32>>, // groups has groups ids as indexes and all the samples id of that group as elements.
    next_group: usize,
    alleles: usize,
    n_samples_tot: usize
}

impl Iterator for IndelRider {
    type Item = Vec<u32>;

    fn next(&mut self) -> Option<Vec<u32>> {
        if self.next_group == self.groups.len() {
            None
        } else {
            let ref res = self.groups[self.next_group];
            self.next_group += 1;
            Some(res.to_vec()) //XXX FIXME but why do we need to clone it? Do we need lifetimes or...?
        }
    }
}

impl IndelRider {
    pub fn new(snps_buffer: & VecDeque<mutations::Mutation>, n_overlapping: u32, n_samples: usize) -> IndelRider {
        let mut groups : Vec<u32> =  vec![0; n_samples*2];
        IndelRider::count_groups(snps_buffer, n_overlapping, & mut groups, n_samples);
        let n_alleles = groups.len();
        let n_groups = groups.iter().max().unwrap();
        let n = *n_groups as usize;
        let mut rev_groups : Vec<Vec<u32>> = vec![Vec::new(); n+1]; // functional way to do this?
        // groups has chr samples as indexes and group ids as elements, we need to invert this array.
        for (sample, group) in groups.iter().enumerate() {
            rev_groups[*group as usize].push(sample as u32); // Mh, use all usize and stop? XXX
        }
        IndelRider{ groups: rev_groups, next_group: 0, alleles: n_alleles, n_samples_tot: n_samples}
    }
        
    /// Function that assigns chr samples to different groups depending on their overlapping indel alleles.
    /// chr in the same group have the same alleles of the same indels, i.e. their coords are in sync.
    ///
    /// # Arguments
    ///
    /// * `snps_buffer`- a mutable reference to the VecDeque that is used as a buffer for SNPs. Should contain the SNPs
    ///    overlapping the bed that we are interested in.
    /// * `n_overlapping` - the number of overlapping SNPs, since buffer will have one more.
    /// * `groups` - a mutable reference to a Vec of u32 that will be filled with groups info. Indexes: samples id. Elements: groups id.
    /// * `n_sample` - the number of samples (each with two alleles for each SNP) for which we have genotypes.
    ///
    /// This needs also to define manage/do not manage
    fn count_groups(snps_buffer: & VecDeque<mutations::Mutation>, n_overlapping: u32, groups: &mut Vec<u32>, n_samples: usize) {
        for (i_snp, snp) in snps_buffer.iter().enumerate() {
            if snp.is_indel && i_snp < n_overlapping as usize { // i >= n_overlapping we have finished the overlapping snps (the last one is just waiting in the buffer)
                if snp.genotypes.iter().any(|x| x.0 || x.1) {
                    // we have a bisection
                    //n_groups = n_groups * 2;
                    // even groups have no indels at this run.
                    for i_sample in 0 .. n_samples {
                        let allele = snp.genotypes[i_sample];
                        let old_group_0 = groups[i_sample];
                        let old_group_1 = groups[i_sample+n_samples];
                        groups[i_sample] = match allele.0 {
                            true => match old_group_0 {
                                0 => 1,
                                _ => 2*old_group_0+1
                            },
                            false => match old_group_0 {
                                0 => 0,
                                _ => 2*old_group_0
                            }
                        };
                        groups[i_sample+n_samples] = match allele.1 {
                            true => match old_group_1 {
                                0 => 1,
                                _ => 2*old_group_1+1,
                            },
                            false => match old_group_1 {
                                0 => 0,
                                _ => 2*old_group_1
                            }
                        };
                    }
                }
            }
        }
    }
        
    // Function that given a group id and a window will return info on the overlapping SNPs for that group and on the resulting window length
    #[allow(unused_variables)]
    #[allow(unused_assignments)]  // WTF
    pub fn get_group_info(&self, window: & mut mutations::Coordinate, snps_buffer: & VecDeque<mutations::Mutation>, n_overlapping: u32, info: & mut Vec<(usize, MutationClass)>) {
        let mut pos : u64 = 0; // relative position inside the window that we are at.
        for (i_snp, snp) in snps_buffer.iter().enumerate() {
            if i_snp < n_overlapping as usize { // i >= n_overlapping we have finished the overlapping snps (the last one is just waiting in the buffer)
                let mut group_genotypes : Vec<&(bool, bool)> = Vec::with_capacity((self.alleles/2) as usize);
                for i_sample in self.groups[self.next_group].to_vec() {
                   let index = i_sample as usize % self.n_samples_tot;
                   group_genotypes.push(snp.genotypes.get(index).unwrap());
                }
                // check overlap, will need another method
                let mut res_mutclass = MutationClass::Manage(pos as usize); // the majority are SNPs so we start with this.
                let mut snp_coords = mutations::Coordinate{ chr: snp.pos.chr.to_owned(), start: snp.pos.start, end: snp.pos.end };
                if group_genotypes.iter().any(|&x| x.0 || x.1) && snp.is_indel { 
                    let mut is_del = false;
                    if snp.is_indel {
                        is_del = true;
                        if snp.indel_len < 0 {
                            snp_coords.end += (-snp.indel_len) as u64;
                            is_del = false;
                        }
                    }
                    if is_del {
                        res_mutclass = MutationClass::Del(snp.indel_len as u64, pos as usize);
                        pos += snp.indel_len as u64; // for deletions we need to account that we have moved over the reference
                        //for del we need to get more reference since we have removed bases.   
                        window.end += snp.indel_len as u64;
                    } else {
                        res_mutclass = MutationClass::Ins(snp.sequence_alt.to_owned(), pos as usize);
                        pos += 1;
                        //for ins we get less reference since we have inserted bases for this group (snp.len is negative for ins)
                        window.end -= (-snp.indel_len) as u64;
                    }
                    // cannot do here together for ins and del due to types mismatches.
                    //window.end += snp.indel_len;
                } else {
                    res_mutclass = MutationClass::Reference;
                    pos += 1;
                    // we have a SNP always reference in this group or an indel always reference.
                }
                // Determine overlap
                let sub_window = mutations::Coordinate{ chr: window.chr.to_owned(), start: window.start+pos, end: window.end};
                match snp_coords.relative_position(&sub_window) {
                    mutations::Position::Before => {},
                    mutations::Position::Overlapping => { info.push((i_snp, res_mutclass)) },
                    mutations::Position::After => { break } 
                }
            }
        }
    }
}